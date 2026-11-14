use crate::config::theme::Palette;
use rusty_lipgloss::{Border, Position, Style};
use serde::{Deserialize, Serialize};

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
    pub fn occupies_space(self) -> bool {
        !matches!(self, Self::None)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct BorderConfig {
    pub kind: BorderType,
    pub color: String,
}

impl Default for BorderConfig {
    fn default() -> Self {
        Self {
            kind: BorderType::Rounded,
            color: "@border".into(),
        }
    }
}

impl BorderConfig {
    pub fn inset(&self) -> u16 {
        if self.kind.occupies_space() { 1 } else { 0 }
    }
}

// --------------------------------------------------
// PADDING
// --------------------------------------------------

#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
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
#[serde(default, deny_unknown_fields)]
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
            foreground: "@fg".into(),
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

        if let Some(background) = effective_background(self.background.as_deref(), None) {
            style = style.background(background);
        }

        style
    }

    pub fn resolve_with_background(&self, parent: Option<&str>) -> Style {
        let mut style = self.build_style();
        if self.background.is_none()
            && let Some(bg) = parent
        {
            style = style.background(bg);
        }
        style
    }
}

// --------------------------------------------------
// CONTAINER STYLE
// Shared by window, sections and modules
// --------------------------------------------------

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct ContainerStyleConfig {
    pub border: BorderConfig,
    pub padding: Padding,
    pub background: Option<String>,
    pub align_h: Alignment,
    pub align_v: Alignment,
}

impl ContainerStyleConfig {
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

#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Alignment {
    #[default]
    #[serde(alias = "left", alias = "top")]
    Start,
    Center,
    #[serde(alias = "right", alias = "bottom")]
    End,
}
impl Alignment {
    pub fn position(self) -> Position {
        match self {
            Self::Start => Position::LEFT,
            Self::Center => Position::CENTER,
            Self::End => Position::RIGHT,
        }
    }
    pub fn offset(self, available: u16, used: u16) -> u16 {
        let spare = available.saturating_sub(used);
        match self {
            Self::Start => 0,
            Self::Center => spare / 2,
            Self::End => spare,
        }
    }
}
pub trait StyleBuilder {
    fn build_style(&self) -> Style;
}

/// Omitted backgrounds inherit; explicit transparency restores the terminal default.
pub fn effective_background<'a>(own: Option<&'a str>, parent: Option<&'a str>) -> Option<&'a str> {
    match own {
        Some("transparent") => None,
        Some(color) => Some(color),
        None => parent,
    }
}

fn resolve_background(value: Option<&str>, palette: &Palette) -> anyhow::Result<Option<String>> {
    value
        .map(|value| {
            if value == "transparent" {
                Ok(value.to_owned())
            } else {
                palette.color(value)
            }
        })
        .transpose()
}
impl StyleBuilder for TextStyleConfig {
    fn build_style(&self) -> Style {
        self.resolve()
    }
}
impl TextStyleConfig {
    pub fn colors(&self, palette: &Palette) -> anyhow::Result<Self> {
        let mut s = self.clone();
        s.foreground = palette.color(&s.foreground)?;
        s.background = resolve_background(s.background.as_deref(), palette)?;
        Ok(s)
    }
}
impl ContainerStyleConfig {
    pub fn plain() -> Self {
        Self {
            border: BorderConfig {
                kind: BorderType::None,
                ..Default::default()
            },
            ..Default::default()
        }
    }
    pub fn colors(&self, palette: &Palette) -> anyhow::Result<Self> {
        let mut s = self.clone();
        s.border.color = palette.color(&s.border.color)?;
        s.background = resolve_background(s.background.as_deref(), palette)?;
        Ok(s)
    }
    pub fn decoration(&self, width: u16, height: u16) -> String {
        let border = match self.border.kind {
            BorderType::None => None,
            BorderType::Hidden => Some(Border::hidden()),
            BorderType::Normal => Some(Border::normal()),
            BorderType::Rounded => Some(Border::rounded()),
            BorderType::Thick => Some(Border::thick()),
            BorderType::Double => Some(Border::double()),
        };
        // This Lip Gloss release includes borders in width and height.
        let mut style = Style::new()
            .width(usize::from(width))
            .height(usize::from(height))
            .max_width(usize::from(width))
            .max_height(usize::from(height));
        if let Some(b) = border {
            style = style
                .border(b, &[true])
                .border_foreground(&[&self.border.color]);
        }
        if let Some(bg) = effective_background(self.background.as_deref(), None) {
            style = style.background(bg);
        }
        style.render("")
    }
}
