//! Allocate terminal rectangles. No collection, styling output, or terminal I/O.
use crate::{
    content::Resolved,
    layout::{Direction, Size, rect::Rect},
};

pub struct Placed<'a> {
    pub node: &'a Resolved,
    pub area: Rect,
    pub content: Rect,
    pub children: Vec<Placed<'a>>,
}

impl Resolved {
    pub fn place(&self, area: Rect) -> Placed<'_> {
        let content = area.inner(&self.container);
        let title = self.title_rows().min(content.height);
        let lines = (self.lines.len().min(u16::MAX as usize) as u16)
            .min(content.height.saturating_sub(title));
        let child_area = Rect {
            y: content.y + title + lines,
            height: content.height.saturating_sub(title + lines),
            ..content
        };

        let mut rects = allocate(&self.children, self.direction, self.gap, child_area);
        if let Some(last) = rects.last() {
            let (dx, dy) = if self.direction == Direction::Horizontal {
                (
                    self.children_align_h
                        .unwrap_or(self.container.align_h)
                        .offset(child_area.width, last.x + last.width - child_area.x),
                    0,
                )
            } else {
                (
                    0,
                    self.children_align_v
                        .unwrap_or(self.container.align_v)
                        .offset(child_area.height, last.y + last.height - child_area.y),
                )
            };

            for rect in &mut rects {
                rect.x += dx;
                rect.y += dy;
            }
        }
        // Along the cross axis, explicit alignment sizes each child to its content.
        // Omitted alignment retains the existing full-width/full-height behavior.
        for (child, rect) in self.children.iter().zip(&mut rects) {
            match self.direction {
                Direction::Vertical => {
                    if let Some(align) = self.children_align_h {
                        let width = child.measure().0.min(rect.width);
                        rect.x += align.offset(rect.width, width);
                        rect.width = width;
                    }
                }
                Direction::Horizontal => {
                    if let Some(align) = self.children_align_v {
                        let height = child.measure().1.min(rect.height);
                        rect.y += align.offset(rect.height, height);
                        rect.height = height;
                    }
                }
            }
        }

        let children = self
            .children
            .iter()
            .zip(rects)
            .map(|(n, r)| n.place(r))
            .collect();
        Placed {
            node: self,
            area,
            content,
            children,
        }
    }
}

pub fn allocate(nodes: &[Resolved], direction: Direction, gap: u16, area: Rect) -> Vec<Rect> {
    let length = if direction == Direction::Horizontal {
        area.width
    } else {
        area.height
    };

    let gap = if nodes.len() > 1 {
        gap.min(length / (nodes.len() - 1).min(u16::MAX as usize) as u16)
    } else {
        0
    };

    let available = length.saturating_sub(
        gap.saturating_mul(nodes.len().saturating_sub(1).min(u16::MAX as usize) as u16),
    );
    let mut remaining = available;
    let mut sizes = vec![0; nodes.len()];
    for (i, node) in nodes.iter().enumerate() {
        let request = match node.size {
            Size::Fill => continue,
            Size::Fixed(n) => n,
            Size::Percent(n) => (u32::from(available) * u32::from(n) / 100) as u16,
            Size::Auto => {
                let (w, h) = node.measure();
                if direction == Direction::Horizontal {
                    w
                } else {
                    h
                }
            }
        };
        sizes[i] = request.min(remaining);
        remaining -= sizes[i];
    }

    let mut fills = nodes.iter().filter(|n| n.size == Size::Fill).count() as u16;
    for (i, node) in nodes.iter().enumerate() {
        if node.size == Size::Fill {
            sizes[i] = remaining / fills;
            remaining -= sizes[i];
            fills -= 1;
        }
    }

    let mut offset = 0u16;
    sizes
        .into_iter()
        .map(|size| {
            let rect = if direction == Direction::Horizontal {
                Rect {
                    x: area.x.saturating_add(offset),
                    width: size,
                    ..area
                }
            } else {
                Rect {
                    y: area.y.saturating_add(offset),
                    height: size,
                    ..area
                }
            };
            offset = offset.saturating_add(size).saturating_add(gap).min(length);
            rect
        })
        .collect()
}
