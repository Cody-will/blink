mod compatibility;
mod document;
mod shorthand;
pub mod theme;
use crate::{
    components,
    layout::{
        Direction, Size,
        node::{Kind, Node},
    },
    styles::{ContainerStyleConfig, Padding, TextStyleConfig},
};
use anyhow::{Context, bail};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};
use theme::Palette;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub version: u16,
    pub name: String,
    pub theme: String,
    pub palette: toml::Table,
    pub reserved_rows: u16,
    pub graphics: Backend,
    pub display: Display,
    pub icons: Icons,
    pub layout: Layout,
    pub styles: Styles,
}
#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize, PartialEq, clap::ValueEnum)]
#[serde(rename_all = "snake_case")]
pub enum Backend {
    #[default]
    Auto,
    Kitty,
    Iterm2,
    Sixel,
    Ansi,
    Text,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Display {
    pub unavailable: Unavailable,
    pub probe_timeout_ms: u16,
    pub probe: bool,
    pub cell_width: u16,
    pub cell_height: u16,
    pub multiplexer_passthrough: bool,
}
impl Default for Display {
    fn default() -> Self {
        Self {
            unavailable: Unavailable::Hide,
            probe_timeout_ms: 50,
            probe: true,
            cell_width: 8,
            cell_height: 16,
            multiplexer_passthrough: false,
        }
    }
}
#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Unavailable {
    #[default]
    Show,
    Hide,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Icons {
    pub enabled: bool,
    pub mode: IconMode,
}
impl Default for Icons {
    fn default() -> Self {
        Self {
            enabled: true,
            mode: IconMode::Unicode,
        }
    }
}
#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum IconMode {
    #[default]
    Unicode,
    Nerd,
    Ascii,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Layout {
    pub direction: Direction,
    pub gap: u16,
    pub first: Node,
    pub second: Node,
}
impl Default for Layout {
    fn default() -> Self {
        Self {
            direction: Direction::Horizontal,
            gap: 1,
            first: Node {
                kind: Kind::Image,
                size: Size::Percent(35),
                optional: true,
                ..Default::default()
            },
            second: Node {
                kind: Kind::Column,
                gap: 1,
                responsive: true,
                header: presentation(&["title", "separator"]),
                footer: presentation(&["break", "colors"]),
                children: ["system", "session", "hardware", "storage"]
                    .into_iter()
                    .filter_map(components::preset)
                    .map(|(title, names)| module(title, names))
                    .collect(),
                ..Default::default()
            },
        }
    }
}
fn presentation(names: &[&str]) -> Vec<Node> {
    names
        .iter()
        .map(|name| Node {
            kind: Kind::Component,
            name: (*name).into(),
            size: Size::Auto,
            ..Default::default()
        })
        .collect()
}
pub fn module(title: &str, names: &[&str]) -> Node {
    Node {
        kind: Kind::Module,
        title: title.into(),
        size: Size::Auto,
        children: names
            .iter()
            .map(|name| Node {
                kind: Kind::Component,
                name: (*name).into(),
                size: Size::Auto,
                optional: matches!(*name, "hostname" | "uptime" | "shell" | "cpu_cores"),
                ..Default::default()
            })
            .collect(),
        ..Default::default()
    }
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Styles {
    pub window: ContainerStyleConfig,
    pub section: ContainerStyleConfig,
    pub module: ContainerStyleConfig,
    pub component: ContainerStyleConfig,
    pub text: TextStyleConfig,
    pub title: TextStyleConfig,
    pub label: TextStyleConfig,
    pub icon: TextStyleConfig,
}
impl Default for Styles {
    fn default() -> Self {
        Self {
            window: ContainerStyleConfig {
                padding: Padding::all(1),
                background: Some("transparent".into()),
                ..Default::default()
            },
            section: ContainerStyleConfig::plain(),
            module: ContainerStyleConfig {
                padding: Padding {
                    left: 1,
                    right: 1,
                    ..Default::default()
                },
                ..Default::default()
            },
            component: ContainerStyleConfig::plain(),
            text: TextStyleConfig::default(),
            title: TextStyleConfig {
                bold: true,
                foreground: "@accent_alt".into(),
                ..Default::default()
            },
            label: TextStyleConfig {
                bold: true,
                foreground: "@accent".into(),
                ..Default::default()
            },
            icon: TextStyleConfig {
                foreground: "@accent_alt".into(),
                ..Default::default()
            },
        }
    }
}
impl Default for Config {
    fn default() -> Self {
        Self {
            version: 1,
            name: "blink".into(),
            theme: "gruvbox-material".into(),
            palette: toml::Table::new(),
            reserved_rows: 2,
            graphics: Backend::Auto,
            display: Display::default(),
            icons: Icons::default(),
            layout: Layout::default(),
            styles: Styles::default(),
        }
    }
}
/// Tables merge recursively; lists and changed node types replace their defaults.
pub fn merge(base: &mut toml::Value, overrides: toml::Value) {
    if base.get("type").is_some()
        && overrides.get("type").is_some()
        && base.get("type") != overrides.get("type")
    {
        *base = overrides;
        return;
    }
    match (base, overrides) {
        (toml::Value::Table(base), toml::Value::Table(values)) => {
            for (key, value) in values {
                if let Some(old) = base.get_mut(&key) {
                    merge(old, value);
                } else {
                    base.insert(key, value);
                }
            }
        }
        (base, value) => *base = value,
    }
}
pub fn override_style<T: Serialize + serde::de::DeserializeOwned>(
    base: &T,
    overrides: &toml::Table,
) -> anyhow::Result<T> {
    let mut value = toml::Value::try_from(base)?;
    merge(&mut value, toml::Value::Table(overrides.clone()));
    value.try_into().context("Invalid style override")
}
impl Config {
    pub fn parse(text: &str) -> anyhow::Result<Self> {
        let mut value: toml::Value = toml::from_str(text).context("Invalid TOML")?;
        compatibility::convert(&mut value)?;
        shorthand::normalize(&mut value)?;
        let mut defaults = toml::Value::try_from(Self::default())?;
        merge(&mut defaults, value);
        let mut cfg: Self = defaults.try_into().context("Invalid Blink configuration")?;
        cfg.expand_presets()?;
        cfg.validate()?;
        Ok(cfg)
    }
    pub fn path(path: Option<&Path>) -> PathBuf {
        path.map(Path::to_path_buf).unwrap_or_else(|| {
            dirs::config_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("blink/config.toml")
        })
    }
    pub fn load(path: Option<&Path>, create: bool) -> anyhow::Result<(Self, PathBuf)> {
        let explicit = path.is_some();
        let path = Self::path(path);
        let cfg = match fs::read_to_string(&path) {
            Ok(text) => {
                Self::parse(&text).with_context(|| format!("Reading {}", path.display()))?
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound && create => {
                if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
                    fs::create_dir_all(parent)?;
                }
                let cfg = Self::default();
                let contents = cfg.editable_toml()?;
                fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(&path)?
                    .write_all(contents.as_bytes())?;
                cfg
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound && !create && !explicit => {
                Self::default()
            }
            Err(e) => return Err(e).with_context(|| format!("Reading {}", path.display())),
        };
        Ok((cfg, path))
    }
    pub fn editable_toml(&self) -> anyhow::Result<String> {
        document::export(self)
    }
    pub fn colors(&self) -> anyhow::Result<Palette> {
        let mut value = toml::Value::try_from(Palette::theme(&self.theme)?)?;
        merge(&mut value, toml::Value::Table(self.palette.clone()));
        let palette: Palette = value.try_into().context("Invalid palette override")?;
        for (_, v) in toml::Value::try_from(&palette)?
            .as_table()
            .context("Invalid palette")?
        {
            theme::validate_color(v.as_str().context("Palette colors must be strings")?)?;
        }
        Ok(palette)
    }
    fn expand_presets(&mut self) -> anyhow::Result<()> {
        fn expand(n: &mut Node) -> anyhow::Result<()> {
            if n.kind == Kind::Component
                && let Some(name) = components::canonical(&n.name)
            {
                n.name = name.into();
            }
            if let Some(preset) = n.preset.take() {
                if n.kind != Kind::Module || !n.children.is_empty() {
                    bail!("Module preset requires a module without explicit children");
                }
                let (title, keys) = components::preset(&preset)
                    .with_context(|| format!("Unknown module preset {preset:?}"))?;
                n.children = module(title, keys).children;
                if n.title.is_empty() {
                    n.title = title.into();
                }
            }
            for c in n
                .header
                .iter_mut()
                .chain(&mut n.children)
                .chain(&mut n.footer)
            {
                expand(c)?;
            }
            Ok(())
        }
        expand(&mut self.layout.first)?;
        expand(&mut self.layout.second)
    }
    pub fn validate(&self) -> anyhow::Result<()> {
        if self.version != 1 {
            bail!("Unsupported configuration version {}", self.version);
        }
        if !(1..=256).contains(&self.display.cell_width)
            || !(1..=256).contains(&self.display.cell_height)
        {
            bail!("Terminal cell dimensions must be between 1 and 256");
        }
        if self.display.probe_timeout_ms > 1000 {
            bail!("probe_timeout_ms must be at most 1000");
        }
        let p = self.colors()?;
        for (name, s) in [
            ("window", &self.styles.window),
            ("section", &self.styles.section),
            ("module", &self.styles.module),
            ("component", &self.styles.component),
        ] {
            s.colors(&p).with_context(|| format!("styles.{name}"))?;
        }
        for (name, s) in [
            ("text", &self.styles.text),
            ("title", &self.styles.title),
            ("label", &self.styles.label),
            ("icon", &self.styles.icon),
        ] {
            s.colors(&p).with_context(|| format!("styles.{name}"))?;
        }
        fn check(
            n: &Node,
            cfg: &Config,
            p: &Palette,
            depth: usize,
            path: &str,
        ) -> anyhow::Result<()> {
            if depth > 64 || n.children.len() > 4096 {
                bail!("{path}: excessive layout depth or children");
            }
            if let Size::Percent(v) = n.size
                && v > 100
            {
                bail!("{path}.size: percentage must be 0–100");
            }
            if n.kind == Kind::Component
                && n.value.is_none()
                && components::definition(&n.name).is_none()
            {
                bail!("{path}.name: unknown component {:?}", n.name);
            }
            if n.kind == Kind::Ascii && n.image != crate::layout::node::ImageOptions::default() {
                bail!("{path}: ASCII art does not use raster image fitting options");
            }
            if n.kind == Kind::Ascii && n.path.is_some() && !n.content.is_empty() {
                bail!("{path}: ASCII art accepts a file path or inline content, not both");
            }
            if n.responsive && n.kind != Kind::Column {
                bail!("{path}: responsive requires a column container");
            }
            if !matches!(n.kind, Kind::Row | Kind::Column | Kind::Module)
                && (!n.children.is_empty() || !n.header.is_empty() || !n.footer.is_empty())
            {
                bail!("{path}: leaf nodes cannot contain children");
            }
            let base = match n.kind {
                Kind::Module => cfg.styles.module.clone(),
                Kind::Component => cfg.styles.component.clone(),
                _ => ContainerStyleConfig::plain(),
            };
            override_style(&base, &n.style)?
                .colors(p)
                .with_context(|| format!("{path}.style"))?;
            for (name, base, table) in [
                ("text_style", &cfg.styles.text, &n.text_style),
                ("label_style", &cfg.styles.label, &n.label_style),
                ("icon_style", &cfg.styles.icon, &n.icon_style),
            ] {
                override_style(base, table)?
                    .colors(p)
                    .with_context(|| format!("{path}.{name}"))?;
            }
            for (i, c) in n
                .header
                .iter()
                .chain(&n.children)
                .chain(&n.footer)
                .enumerate()
            {
                check(c, cfg, p, depth + 1, &format!("{path}.children[{i}]"))?;
            }
            Ok(())
        }
        check(&self.layout.first, self, &p, 0, "layout.first")?;
        check(&self.layout.second, self, &p, 0, "layout.second")
    }
}
