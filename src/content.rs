//! Resolve configuration and a system snapshot into displayable content. No terminal I/O.
use crate::{
    components::{self, system::Snapshot},
    config::{Config, Unavailable, override_style},
    graphics::assets::Assets,
    layout::{
        Direction, Size,
        node::{ImageOptions, Kind, Node},
    },
    styles::{Alignment, ContainerStyleConfig, TextStyleConfig},
};
use image::DynamicImage;
use std::{path::Path, rc::Rc};
use unicode_width::UnicodeWidthStr;

pub type Line = Vec<(String, TextStyleConfig)>;
#[derive(Clone)]
pub struct Resolved {
    pub kind: Kind,
    pub responsive: bool,
    pub optional: bool,
    pub role: Role,
    pub component_name: Option<String>,
    pub size: Size,
    pub direction: Direction,
    pub gap: u16,
    pub children_align_h: Option<Alignment>,
    pub children_align_v: Option<Alignment>,
    pub container: ContainerStyleConfig,
    pub lines: Vec<Line>,
    pub children: Vec<Resolved>,
    pub image: Option<Rc<DynamicImage>>,
    pub image_options: ImageOptions,
    pub title: Option<(String, TextStyleConfig)>,
    pub title_align: Alignment,
    pub component: Option<ComponentRow>,
    pub swatches: bool,
    pub cell_ratio: f64,
    pub image_columns: u16,
}
#[derive(Clone)]
pub struct ComponentRow {
    pub icon: String,
    pub label: String,
    pub value: String,
    pub icon_style: TextStyleConfig,
    pub label_style: TextStyleConfig,
    pub value_style: TextStyleConfig,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Header,
    Body,
    Footer,
}
impl Resolved {
    pub fn empty(container: ContainerStyleConfig) -> Self {
        Self {
            kind: Kind::Column,
            responsive: false,
            optional: false,
            role: Role::Body,
            component_name: None,
            size: Size::Fill,
            direction: Direction::Vertical,
            gap: 0,
            children_align_h: None,
            children_align_v: None,
            container,
            lines: vec![],
            children: vec![],
            image: None,
            image_options: ImageOptions::default(),
            title: None,
            title_align: Alignment::Center,
            component: None,
            swatches: false,
            cell_ratio: 2.0,
            image_columns: 20,
        }
    }
    pub fn title_rows(&self) -> u16 {
        u16::from(self.title.is_some() && !self.container.border.kind.occupies_space())
    }
}
/// Hide optional visuals only when explicitly marked optional. Custom arrangements never reflow.
pub fn prepare(
    cfg: &mut Config,
    width: u16,
    height: u16,
    no_image: bool,
    no_icons: bool,
    plain: bool,
) {
    if no_icons || plain {
        cfg.icons.enabled = false;
    }
    fn visit(
        n: &mut Node,
        width: u16,
        height: u16,
        no_image: bool,
        plain: bool,
        in_responsive: bool,
    ) {
        let responsive = in_responsive || n.responsive;
        if n.kind == Kind::Image && (no_image || plain || n.optional && (width < 64 || height < 20))
        {
            n.visible = false;
        }
        if n.kind == Kind::Ascii && n.optional && !plain && (width < 64 || height < 20) {
            n.visible = false;
        }
        if n.optional
            && n.kind == Kind::Swatches
            && (plain || !responsive && (height < 24 || width < 64))
        {
            n.visible = false;
        }
        if !plain
            && !responsive
            && n.optional
            && n.kind == Kind::Component
            && (width < 40 || height < 14)
        {
            n.visible = false;
        }
        if no_image || plain {
            n.icon_path = None;
        }
        for child in n
            .header
            .iter_mut()
            .chain(&mut n.children)
            .chain(&mut n.footer)
        {
            visit(child, width, height, no_image, plain, responsive);
        }
    }
    visit(&mut cfg.layout.first, width, height, no_image, plain, false);
    visit(
        &mut cfg.layout.second,
        width,
        height,
        no_image,
        plain,
        false,
    );
}
pub fn resolve(
    cfg: &Config,
    snapshot: &Snapshot,
    base: &Path,
    assets: &mut Assets,
) -> anyhow::Result<Resolved> {
    let palette = cfg.colors()?;
    fn build(
        n: &Node,
        cfg: &Config,
        snapshot: &Snapshot,
        base: &Path,
        assets: &mut Assets,
        p: &crate::config::theme::Palette,
    ) -> anyhow::Result<Option<Resolved>> {
        if !n.visible {
            return Ok(None);
        }
        let base_style = match n.kind {
            Kind::Module => cfg.styles.module.clone(),
            Kind::Component => cfg.styles.component.clone(),
            _ => ContainerStyleConfig::plain(),
        };
        let mut r = Resolved::empty(override_style(&base_style, &n.style)?.colors(p)?);
        r.cell_ratio = f64::from(cfg.display.cell_height) / f64::from(cfg.display.cell_width);
        r.kind = n.kind;
        r.responsive = n.responsive;
        r.optional = n.optional;
        if n.kind == Kind::Component && n.value.is_none() {
            r.component_name = Some(n.name.clone());
        }
        r.size = n.size;
        r.gap = n.gap;
        r.children_align_h = n.align_h;
        r.children_align_v = n.align_v;
        r.title_align = n.title_align;
        r.direction = match n.kind {
            Kind::Row => Direction::Horizontal,
            Kind::Column => Direction::Vertical,
            _ => n.direction,
        };
        let text = override_style(&cfg.styles.text, &n.text_style)?.colors(p)?;
        if !n.title.is_empty() {
            r.title = Some((clean(&n.title), cfg.styles.title.colors(p)?));
        }
        match n.kind {
            Kind::Text => {
                for line in clean(&n.content).split('\n') {
                    r.lines.push(vec![(line.into(), text.clone())]);
                }
            }
            Kind::Ascii => {
                let art = if let Some(path) = &n.path {
                    assets
                        .load_ascii(path, base)
                        .unwrap_or_else(|| Rc::from("[art unavailable]"))
                } else {
                    Rc::from(crate::graphics::assets::ascii(&n.content))
                };
                for line in art.split('\n') {
                    r.lines.push(vec![(line.into(), text.clone())]);
                }
            }
            Kind::Component => {
                let presentation = if n.value.is_none() {
                    components::definition(&n.name)
                        .map(|d| d.presentation())
                        .unwrap_or(components::Presentation::Data)
                } else {
                    components::Presentation::Data
                };
                match presentation {
                    components::Presentation::Title | components::Presentation::Separator => {
                        let title = snapshot.formatted("title");
                        if title.is_none() && cfg.display.unavailable == Unavailable::Hide {
                            return Ok(None);
                        }
                        let title = clean(title.as_deref().unwrap_or("N/A"));
                        let output = if presentation == components::Presentation::Separator {
                            "─".repeat(title.width().min(512))
                        } else {
                            title
                        };
                        r.lines.push(vec![(output, cfg.styles.title.colors(p)?)]);
                    }
                    components::Presentation::Break => r.lines.push(vec![(String::new(), text)]),
                    components::Presentation::Colors => {
                        r.swatches = true;
                        for start in [0, 8] {
                            r.lines.push(
                                (start..start + 8)
                                    .map(|index| {
                                        (
                                            "██".into(),
                                            TextStyleConfig {
                                                foreground: index.to_string(),
                                                ..text.clone()
                                            },
                                        )
                                    })
                                    .collect(),
                            );
                        }
                    }
                    components::Presentation::Data => {
                        let value = n.value.clone().or_else(|| snapshot.formatted(&n.name));
                        if value.is_none() && cfg.display.unavailable == Unavailable::Hide {
                            return Ok(None);
                        }
                        let label = n.label.as_deref().unwrap_or_else(|| {
                            components::definition(&n.name)
                                .map(|d| d.label)
                                .unwrap_or(&n.name)
                        });
                        let icon = if cfg.icons.enabled {
                            n.icon
                                .as_deref()
                                .unwrap_or_else(|| components::icon(&n.name, cfg.icons.mode))
                        } else {
                            ""
                        };
                        r.component = Some(ComponentRow {
                            icon: clean(icon).replace('\n', ""),
                            label: clean(label).replace('\n', ""),
                            value: clean(value.as_deref().unwrap_or("N/A")),
                            icon_style: override_style(&cfg.styles.icon, &n.icon_style)?
                                .colors(p)?,
                            label_style: override_style(&cfg.styles.label, &n.label_style)?
                                .colors(p)?,
                            value_style: text,
                        });
                        // Raster icons have their own bounded sibling region, never print file paths as glyphs.
                        if cfg.icons.enabled
                            && let Some(path) = &n.icon_path
                        {
                            let mut icon = Resolved::empty(ContainerStyleConfig::plain());
                            icon.image = assets.load(Some(path), base);
                            icon.size = Size::Fixed(2);
                            icon.image_columns = 2;
                            if icon.image.is_some() {
                                r.children.push(icon);
                                r.direction = Direction::Horizontal;
                            }
                        }
                    }
                }
            }
            Kind::Image => {
                r.image = assets.load(n.path.as_deref(), base);
                r.image_options = n.image.clone();
                if r.image.is_none() {
                    r.lines.push(vec![("[image unavailable]".into(), text)]);
                }
            }
            Kind::Swatches => {
                let swatch_style = cfg.styles.text.colors(p)?;
                r.swatches = true;
                r.lines.push(
                    [
                        &p.fg,
                        &p.muted,
                        &p.accent,
                        &p.accent_alt,
                        &p.success,
                        &p.warning,
                        &p.error,
                    ]
                    .into_iter()
                    .map(|c| {
                        (
                            "■ ".into(),
                            TextStyleConfig {
                                foreground: c.clone(),
                                ..swatch_style.clone()
                            },
                        )
                    })
                    .collect(),
                );
            }
            _ => {}
        }
        for (nodes, role) in [
            (&n.header, Role::Header),
            (&n.children, Role::Body),
            (&n.footer, Role::Footer),
        ] {
            for child in nodes {
                if let Some(mut child) = build(child, cfg, snapshot, base, assets, p)? {
                    child.role = role;
                    r.children.push(child);
                }
            }
        }
        if n.kind == Kind::Module
            && !n.children.is_empty()
            && r.children.is_empty()
            && cfg.display.unavailable == Unavailable::Hide
        {
            return Ok(None);
        }
        if matches!(n.kind, Kind::Row | Kind::Column) && r.children.is_empty() && r.title.is_none()
        {
            return Ok(None);
        }
        align_components(&mut r);
        if r.component.is_some() {
            format_component(&mut r, 0, 0);
        }
        Ok(Some(r))
    }
    let mut root = Resolved::empty(cfg.styles.window.colors(&palette)?);
    root.direction = cfg.layout.direction;
    root.gap = cfg.layout.gap;
    root.title = Some((clean(&cfg.name), cfg.styles.title.colors(&palette)?));
    for n in [&cfg.layout.first, &cfg.layout.second] {
        if let Some(mut child) = build(n, cfg, snapshot, base, assets, &palette)? {
            let mut section = Resolved::empty(cfg.styles.section.colors(&palette)?);
            section.size = child.size;
            section.optional = child.optional;
            child.size = Size::Fill;
            section.children.push(child);
            root.children.push(section);
        }
    }
    // A lone surviving section consumes all available space after optional content is removed.
    if root.children.len() == 1 {
        root.children[0].size = Size::Fill;
    }
    Ok(root)
}
fn align_components(module: &mut Resolved) {
    let icon_width = module
        .children
        .iter()
        .filter_map(|c| c.component.as_ref())
        .map(|c| c.icon.width())
        .max()
        .unwrap_or(0);
    let label_width = module
        .children
        .iter()
        .filter_map(|c| c.component.as_ref())
        .map(|c| c.label.width())
        .max()
        .unwrap_or(0);
    for child in &mut module.children {
        if child.component.is_some() {
            format_component(child, icon_width, label_width);
        }
    }
}
fn format_component(node: &mut Resolved, icon_width: usize, label_width: usize) {
    let Some(c) = &node.component else {
        return;
    };
    let iw = icon_width.max(c.icon.width());
    let lw = label_width.max(c.label.width());
    node.lines = c
        .value
        .split('\n')
        .enumerate()
        .map(|(i, value)| {
            let mut line = vec![];
            if iw > 0 {
                let icon = if i == 0 { &c.icon } else { "" };
                line.push((
                    format!("{icon}{} ", " ".repeat(iw.saturating_sub(icon.width()))),
                    c.icon_style.clone(),
                ));
            }
            if lw > 0 {
                let label = if i == 0 { &c.label } else { "" };
                line.push((
                    format!("{label}{}  ", " ".repeat(lw.saturating_sub(label.width()))),
                    c.label_style.clone(),
                ));
            }
            line.push((value.into(), c.value_style.clone()));
            line
        })
        .collect();
    if !node.children.is_empty() {
        // Place the file icon next to the complete text row, then keep the component's outer style.
        let mut text = Resolved::empty(ContainerStyleConfig::plain());
        text.lines = std::mem::take(&mut node.lines);
        node.children.truncate(1);
        node.children.push(text);
    }
}
pub fn clean(text: &str) -> String {
    // Strip ANSI/control sequences from user content before styling it ourselves.
    rusty_x_ansi::util::strip(text)
        .chars()
        .filter(|c| !c.is_control() || *c == '\n')
        .collect()
}
