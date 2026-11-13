use rusty_lipgloss::{Border, Style};
use serde::{Deserialize, Serialize};

use crate::{
    config::theme::Config,
    terminal::Terminal,
};

// --------------------------------------------------
// BORDER
// --------------------------------------------------

#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BorderType {
    None,
    Normal,
    #[default]
    Rounded,
    Thick,
    Double,
    Hidden,
}

impl BorderType {
    pub fn to_border(self) -> Option<Border> {
        match self {
            Self::None => None,
            Self::Normal => Some(Border::normal()),
            Self::Rounded => Some(Border::rounded()),
            Self::Thick => Some(Border::thick()),
            Self::Double => Some(Border::double()),
            Self::Hidden => Some(Border::hidden()),
        }
    }

    pub fn is_visible(self) -> bool {
        !matches!(self, Self::None | Self::Hidden)
    }

    pub fn occupies_space(self) -> bool {
        !matches!(self, Self::None)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct BorderConfig {
    pub kind: BorderType,
    pub color: String,
}

impl Default for BorderConfig {
    fn default() -> Self {
        Self {
            kind: BorderType::Rounded,
            color: "#a89984".into(),
        }
    }
}

impl BorderConfig {
    pub fn apply(&self, style: Style) -> Style {
        match self.kind.to_border() {
            Some(border) => style
                .border(border, &[true])
                .border_foreground(&[&self.color]),

            None => style,
        }
    }

    pub fn inset(&self) -> u16 {
        if self.kind.occupies_space() {
            1
        } else {
            0
        }
    }
}

// --------------------------------------------------
// PADDING
// --------------------------------------------------

#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct Padding {
    pub top: u16,
    pub bottom: u16,
    pub left: u16,
    pub right: u16,
}

impl Padding {
    pub fn all(value: u16) -> Self {
        Self {
            top: value,
            bottom: value,
            left: value,
            right: value,
        }
    }

    pub fn x_y(x: u16, y: u16) -> Self {
        Self {
            top: y,
            bottom: y,
            left: x,
            right: x,
        }
    }

    pub fn horizontal(&self) -> u16 {
        self.left.saturating_add(self.right)
    }

    pub fn vertical(&self) -> u16 {
        self.top.saturating_add(self.bottom)
    }
}

// --------------------------------------------------
// TEXT STYLE
// --------------------------------------------------

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct TextStyleConfig {
    pub foreground: String,
    pub background: Option<String>,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
}

impl Default for TextStyleConfig {
    fn default() -> Self {
        Self {
            foreground: "#ddc7a1".into(),
            background: None,
            bold: false,
            italic: false,
            underline: false,
        }
    }
}

impl TextStyleConfig {
    pub fn resolve(&self) -> Style {
        let mut style = Style::new()
            .foreground(&self.foreground)
            .bold(self.bold)
            .italic(self.italic)
            .underline(self.underline);

        if let Some(background) = &self.background {
            style = style.background(background);
        }

        style
    }
}

// --------------------------------------------------
// CONTAINER STYLE
// Shared by window, sections and modules
// --------------------------------------------------

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct ContainerStyleConfig {
    pub border: BorderConfig,
    pub padding: Padding,
    pub background: Option<String>,
}


impl ContainerStyleConfig {
    pub fn resolve(&self) -> Style {
        let mut style = Style::new();

        if let Some(background) = &self.background {
            style = style.background(background);
        }

        self.border.apply(style)
    }

    pub fn horizontal_inset(&self) -> u16 {
        self.padding
            .horizontal()
            .saturating_add(self.border.inset().saturating_mul(2))
    }

    pub fn vertical_inset(&self) -> u16 {
        self.padding
            .vertical()
            .saturating_add(self.border.inset().saturating_mul(2))
    }
}

// --------------------------------------------------
// WINDOW
// --------------------------------------------------

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct WindowStyleConfig {
    pub container: ContainerStyleConfig,
}


#[derive(Debug, Clone)]
pub struct WindowStyle {
    pub style: Style,
    pub config: WindowStyleConfig,
}

impl WindowStyle {
    pub fn new(cfg: &Config, terminal: &Terminal) -> Self {
        let config = cfg.window.clone();

        let style = config
            .container
            .resolve()
            .width(terminal.width.into())
            .height(terminal.height.into());

        Self { style, config }
    }
}

// --------------------------------------------------
// LAYOUT SECTIONS
// Left / Right or Top / Bottom
// --------------------------------------------------

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct SectionStyleConfig {
    pub container: ContainerStyleConfig,
}

impl Default for SectionStyleConfig {
    fn default() -> Self {
        Self {
            container: ContainerStyleConfig {
                border: BorderConfig {
                    kind: BorderType::None,
                    ..BorderConfig::default()
                },
                padding: Padding::default(),
                background: None,
            },
        }
    }
}

#[derive(Debug, Clone)]
pub struct SectionStyle {
    pub style: Style,
    pub config: SectionStyleConfig,
}

impl SectionStyle {
    pub fn new(config: SectionStyleConfig) -> Self {
        let style = config.container.resolve();

        Self { style, config }
    }
}

// --------------------------------------------------
// MODULE
// A container holding multiple components
// --------------------------------------------------

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct ModuleStyleConfig {
    pub container: ContainerStyleConfig,
    pub title: TextStyleConfig,
    pub gap: u16,
}

impl Default for ModuleStyleConfig {
    fn default() -> Self {
        Self {
            container: ContainerStyleConfig {
                border: BorderConfig::default(),
                padding: Padding::all(1),
                background: None,
            },
            title: TextStyleConfig {
                bold: true,
                ..TextStyleConfig::default()
            },
            gap: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ModuleStyle {
    pub container: Style,
    pub title: Style,
    pub padding: Padding,
    pub gap: u16,
    pub config: ModuleStyleConfig,
}

impl ModuleStyle {
    pub fn new(config: ModuleStyleConfig) -> Self {
        let container = config.container.resolve();
        let title = config.title.resolve();
        let padding = config.container.padding;
        let gap = config.gap;

        Self {
            container,
            title,
            padding,
            gap,
            config,
        }
    }
}

// --------------------------------------------------
// COMPONENT
// Individual information row
// Example: CPU: AMD Ryzen 5
// --------------------------------------------------

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct ComponentStyleConfig {
    pub icon: TextStyleConfig,
    pub label: TextStyleConfig,
    pub content: TextStyleConfig,
    pub padding: Padding,
}

impl Default for ComponentStyleConfig {
    fn default() -> Self {
        Self {
            icon: TextStyleConfig::default(),
            label: TextStyleConfig {
                bold: true,
                ..TextStyleConfig::default()
            },
            content: TextStyleConfig::default(),
            padding: Padding::default(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ComponentStyle {
    pub icon: Style,
    pub label: Style,
    pub content: Style,
    pub padding: Padding,
}

impl ComponentStyle {
    pub fn new(cfg: &Config) -> Self {
        let config = ComponentStyleConfig {
            icon: TextStyleConfig {
                foreground: cfg.palette.accent.clone(),
                ..TextStyleConfig::default()
            },
            label: TextStyleConfig {
                foreground: cfg.palette.accent_alt.clone(),
                bold: true,
                ..TextStyleConfig::default()
            },
            content: TextStyleConfig {
                foreground: cfg.palette.fg.clone(),
                ..TextStyleConfig::default()
            },
            ..ComponentStyleConfig::default()
        };

        Self::from_config(&config)
    }

    pub fn from_config(config: &ComponentStyleConfig) -> Self {
        Self {
            icon: config.icon.resolve(),
            label: config.label.resolve(),
            content: config.content.resolve(),
            padding: config.padding,
        }
    }
}

// --------------------------------------------------
// COMBINED LAYOUT STYLES
// --------------------------------------------------

#[derive(Debug, Clone)]
pub struct LayoutStyle {
    pub window: WindowStyle,
    pub first_section: SectionStyle,
    pub second_section: SectionStyle,
    pub module: ModuleStyle,
    pub component: ComponentStyle,
}

impl LayoutStyle {
    pub fn new(cfg: &Config, terminal: &Terminal) -> Self {
        Self {
            window: WindowStyle::new(cfg, terminal),

            first_section: SectionStyle::new(
                cfg.first_section.clone(),
            ),

            second_section: SectionStyle::new(
                cfg.second_section.clone(),
            ),

            module: ModuleStyle::new(
                cfg.module.clone(),
            ),

            component: ComponentStyle::from_config(
                &cfg.component,
            ),
        }
    }
}
