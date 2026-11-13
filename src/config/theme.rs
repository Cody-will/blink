use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{ErrorKind, Write},
    path::Path,
};

use crate::{
    layout::Layout,
    styles::{
        BorderConfig,
        BorderType,
        ComponentStyleConfig,
        ContainerStyleConfig,
        ModuleStyleConfig,
        Padding,
        SectionStyleConfig,
        WindowStyleConfig,
    },
};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct Config {
    pub name: String,
    pub theme: String,
    pub palette: Palette,

    pub layout: Layout,

    pub window: WindowStyleConfig,
    pub first_section: SectionStyleConfig,
    pub second_section: SectionStyleConfig,
    pub module: ModuleStyleConfig,
    pub component: ComponentStyleConfig,
}

impl Default for Config {
    fn default() -> Self {
        let palette = Palette::default();

        Self {
            name: "blink".into(),
            theme: "gruvbox".into(),

            layout: Layout::default(),

            window: WindowStyleConfig {
                container: ContainerStyleConfig {
                    border: BorderConfig {
                        kind: BorderType::Rounded,
                        color: palette.border.clone(),
                    },
                    padding: Padding::all(1),
                    background: Some(palette.bg.clone()),
                },
            },

            first_section: SectionStyleConfig::default(),
            second_section: SectionStyleConfig::default(),

            module: ModuleStyleConfig {
                container: ContainerStyleConfig {
                    border: BorderConfig {
                        kind: BorderType::Rounded,
                        color: palette.border.clone(),
                    },
                    padding: Padding::all(1),
                    background: None,
                },
                ..ModuleStyleConfig::default()
            },

            component: ComponentStyleConfig::default(),

            palette,
        }
    }
}

impl Config {
    /// Loads ~/.config/blink/config.toml.
    /// Creates a default config if one does not exist.
    pub fn load() -> anyhow::Result<Self> {
        let Some(config_dir) = dirs::config_dir() else {
            return Ok(Self::default());
        };

        let path = config_dir.join("blink").join("config.toml");

        let contents = match fs::read_to_string(&path) {
            Ok(contents) => contents,

            Err(err) if err.kind() == ErrorKind::NotFound => {
                let config = Self::default();
                Self::create_config(&path, &config)?;
                return Ok(config);
            }

            Err(err) => return Err(err.into()),
        };

        let config: Self = toml::from_str(&contents)?;

        Ok(config)
    }

    /// Creates the default configuration file.
    fn create_config(
        path: &Path,
        config: &Self,
    ) -> anyhow::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let contents = toml::to_string_pretty(config)?;

        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)?;

        file.write_all(contents.as_bytes())?;

        Ok(())
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct Palette {
    pub fg: String,
    pub bg: String,
    pub border: String,
    pub accent: String,
    pub accent_alt: String,
}

impl Default for Palette {
    fn default() -> Self {
        Self {
            fg: "#ebdbb2".into(),
            bg: "#282828".into(),
            border: "#928374".into(),
            accent: "#d79921".into(),
            accent_alt: "#458588".into(),
        }
    }
}

