use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};

pub const NAMES: &[&str] = &[
    "gruvbox-material",
    "gruvbox-dark",
    "catppuccin-mocha",
    "tokyo-night",
    "nord",
    "dracula",
    "monochrome",
];

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Palette {
    pub fg: String,
    pub bg: String,
    pub muted: String,
    pub border: String,
    pub accent: String,
    pub accent_alt: String,
    pub success: String,
    pub warning: String,
    pub error: String,
}

impl Palette {
    pub fn theme(name: &str) -> Result<Self> {
        let c = match name {
            "gruvbox-material" | "gruvbox-material-dark" => [
                "#d4be98", "#282828", "#a89984", "#665c54", "#d8a657", "#7daea3", "#a9b665",
                "#e78a4e", "#ea6962",
            ],
            "gruvbox" | "gruvbox-dark" => [
                "#ebdbb2", "#282828", "#a89984", "#928374", "#d79921", "#458588", "#b8bb26",
                "#fabd2f", "#fb4934",
            ],
            "catppuccin-mocha" => [
                "#cdd6f4", "#1e1e2e", "#a6adc8", "#585b70", "#cba6f7", "#89b4fa", "#a6e3a1",
                "#f9e2af", "#f38ba8",
            ],
            "tokyo-night" => [
                "#c0caf5", "#1a1b26", "#565f89", "#414868", "#7aa2f7", "#bb9af7", "#9ece6a",
                "#e0af68", "#f7768e",
            ],
            "nord" => [
                "#eceff4", "#2e3440", "#d8dee9", "#4c566a", "#88c0d0", "#81a1c1", "#a3be8c",
                "#ebcb8b", "#bf616a",
            ],
            "dracula" => [
                "#f8f8f2", "#282a36", "#6272a4", "#44475a", "#bd93f9", "#8be9fd", "#50fa7b",
                "#f1fa8c", "#ff5555",
            ],
            "monochrome" => [
                "#eeeeee", "#111111", "#aaaaaa", "#777777", "#ffffff", "#cccccc", "#dddddd",
                "#bbbbbb", "#ffffff",
            ],
            _ => bail!("Unknown theme {name:?}; choose {}", NAMES.join(", ")),
        };
        Ok(Self {
            fg: c[0].into(),
            bg: c[1].into(),
            muted: c[2].into(),
            border: c[3].into(),
            accent: c[4].into(),
            accent_alt: c[5].into(),
            success: c[6].into(),
            warning: c[7].into(),
            error: c[8].into(),
        })
    }

    pub fn color(&self, value: &str) -> Result<String> {
        let result = match value.strip_prefix('@') {
            None => value,
            Some("fg") => &self.fg,
            Some("bg") => &self.bg,
            Some("muted") => &self.muted,
            Some("border") => &self.border,
            Some("accent") => &self.accent,
            Some("accent_alt") => &self.accent_alt,
            Some("success") => &self.success,
            Some("warning") => &self.warning,
            Some("error") => &self.error,
            Some(name) => bail!("Unknown palette reference @{name}"),
        };
        validate_color(result)?;
        Ok(
            if let Some(hex) = result.strip_prefix('#').filter(|s| s.len() == 3) {
                format!("#{}", hex.chars().flat_map(|c| [c, c]).collect::<String>())
            } else {
                result.to_owned()
            },
        )
    }
}

pub fn validate_color(value: &str) -> Result<()> {
    let hex = value
        .strip_prefix('#')
        .is_some_and(|s| matches!(s.len(), 3 | 6) && s.bytes().all(|c| c.is_ascii_hexdigit()));
    if !hex && value.parse::<u8>().is_err() {
        bail!("Invalid color {value:?}; use #RGB, #RRGGBB, 0–255, or a palette reference");
    }
    Ok(())
}
