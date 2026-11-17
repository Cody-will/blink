//! Adapt only explicitly responsive render sections; configuration and snapshots are untouched.
use crate::{
    components,
    content::{Resolved, Role},
    layout::{Direction, Size, node::Kind, rect::Rect},
    styles::ContainerStyleConfig,
};

fn group(children: Vec<Resolved>, role: Role) -> Resolved {
    let mut group = Resolved::empty(ContainerStyleConfig::plain());
    group.size = Size::Auto;
    group.children = children;
    group.role = role;
    group
}

fn candidate(section: &Resolved, columns: usize) -> Resolved {
    let mut result = section.clone();
    result.responsive = false;
    result.children.clear();
    let headers = section
        .children
        .iter()
        .filter(|n| n.role == Role::Header)
        .cloned()
        .collect::<Vec<_>>();
    let footers = section
        .children
        .iter()
        .filter(|n| n.role == Role::Footer)
        .cloned()
        .collect::<Vec<_>>();
    if !headers.is_empty() {
        result.children.push(group(headers, Role::Header));
    }

    let mut lanes = (0..columns)
        .map(|_| {
            let mut lane = group(vec![], Role::Body);
            lane.gap = section.gap;
            lane.size = Size::Fill;
            lane.children_align_h = section.children_align_h;
            lane
        })
        .collect::<Vec<_>>();
    let mut heights = vec![0u32; columns];
    for child in section.children.iter().filter(|n| n.role == Role::Body) {
        let index = heights
            .iter()
            .enumerate()
            .min_by_key(|(i, height)| (**height, *i))
            .map(|(i, _)| i)
            .unwrap_or(0);
        if !lanes[index].children.is_empty() {
            heights[index] += u32::from(section.gap);
        }

        let height = match child.size {
            Size::Fixed(n) => n,
            _ => child.measure().1,
        };
        heights[index] += u32::from(height);
        lanes[index].children.push(child.clone());
    }
    lanes.retain(|lane| !lane.children.is_empty());
    if !lanes.is_empty() {
        let mut grid = group(lanes, Role::Body);
        grid.direction = Direction::Horizontal;
        grid.gap = section.gap;
        // All lanes use the same available height; explicit child sizes are retained.
        result.children.push(grid);
    }

    if !footers.is_empty() {
        result.children.push(group(footers, Role::Footer));
    }
    result
}

fn packing(section: &Resolved, area: Rect) -> (Resolved, bool) {
    let inner = area.inner(&section.container);
    let count = usize::from(
        ((u32::from(inner.width) + u32::from(section.gap)) / (28 + u32::from(section.gap)))
            .clamp(1, 3) as u16,
    );
    let mut best = None;
    for columns in 1..=count {
        let result = candidate(section, columns);
        let height = result.measure().1;
        if height <= area.height {
            return (result, true);
        }

        if best.as_ref().is_none_or(|(_, old)| height < *old) {
            best = Some((result, height));
        }
    }
    (
        best.map(|(n, _)| n).unwrap_or_else(|| section.clone()),
        false,
    )
}

fn fits(node: &Resolved, area: Rect) -> bool {
    if node.responsive {
        return packing(node, area).1;
    }

    let placed = node.place(area);
    node.children
        .iter()
        .zip(&placed.children)
        .all(|(child, p)| fits(child, p.area))
}

fn has_artwork(node: &Resolved) -> bool {
    matches!(node.kind, Kind::Image | Kind::Ascii)
        || node.image.is_some()
        || node.children.iter().any(has_artwork)
}

fn priority(node: &Resolved) -> Option<u8> {
    node.component_name
        .as_deref()
        .and_then(components::definition)
        .map(|d| d.priority())
        .filter(|p| *p < 100)
}

fn lowest(node: &Resolved) -> Option<usize> {
    fn visit(n: &Resolved, index: &mut usize, best: &mut Option<(u8, usize)>, active: bool) {
        let active = active || n.responsive;
        let current = *index;
        *index += 1;
        if active
            && let Some(p) = priority(n)
            && best.is_none_or(|(old, i)| p < old || p == old && current > i)
        {
            *best = Some((p, current));
        }

        for child in &n.children {
            visit(child, index, best, active);
        }
    }

    let mut best = None;
    visit(node, &mut 0, &mut best, false);
    best.map(|(_, i)| i)
}

fn remove(node: &mut Resolved, target: usize, index: &mut usize, active: bool) {
    let active = active || node.responsive;
    *index += 1;
    node.children.retain_mut(|child| {
        if *index == target {
            *index += 1;
            false
        } else {
            remove(child, target, index, active);
            true
        }
    });
    if active {
        node.children.retain(|child| {
            child.kind != Kind::Module || !child.children.is_empty() || !child.lines.is_empty()
        });
    }
}

fn apply(node: &mut Resolved, area: Rect) {
    if node.responsive {
        *node = packing(node, area).0;
    }

    let areas = node
        .place(area)
        .children
        .iter()
        .map(|p| p.area)
        .collect::<Vec<_>>();
    for (child, area) in node.children.iter_mut().zip(areas) {
        apply(child, area);
    }
}

pub fn adapt(root: &mut Resolved, area: Rect, color: bool) {
    fn hide_colors(n: &mut Resolved) {
        n.children.retain(|c| !c.swatches);
        for child in &mut n.children {
            hide_colors(child);
        }
    }

    if !color {
        hide_colors(root);
    }

    if !fits(root, area) {
        root.children
            .retain(|child| !(child.optional && has_artwork(child)));
        if root.children.len() == 1 {
            root.children[0].size = Size::Fill;
        }
    }

    while !fits(root, area) {
        let Some(target) = lowest(root) else {
            break;
        };
        remove(root, target, &mut 0, false);
    }
    apply(root, area);
}
