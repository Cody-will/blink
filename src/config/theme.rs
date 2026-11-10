use serde::{Deserialize, Serialize}; 
use std::{fs, io::{ErrorKind, Write}, path::PathBuf};

#[derive(Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct Config {
    pub name: String,
    pub theme: String,
    pub palette: Palette, 
    pub border: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            name: "blink".into(),
            theme: "gruvbox".into(),
            palette: Palette::default(),
            border: true,
        } 
    }
}

impl Config {
    /// Loads config from users config directory /blink/config.toml or returns default if not found.
    pub fn load() -> anyhow::Result<Config> {
        let Some(config_dir) = dirs::config_dir() else {
            return Ok(Self::default());
        };

        let path = config_dir.join("blink").join("config.toml");

        if !path.exists() {
            Self::create_config(&path)?;
        }
 
        let contents = match fs::read_to_string(&path) {
            Ok(contents) => contents,
            Err(err) if err.kind() == ErrorKind::NotFound => { 
                return Ok(Config::default());
            }
            Err(err) => return Err(err.into()),
        };

        let config: Self = toml::from_str(&contents)?;
        println!("Loading config file at: {}", path.display());

        Ok(config)
    }
    
    /// Creates config directory at .config/blink then creates a base config.toml file
    fn create_config(path: &PathBuf) -> anyhow::Result<()> {
        let contents = toml::to_string_pretty(&Self::default())?;

        if let Some(parent) = path.parent() {
           fs::create_dir_all(parent)?; 
        };
        
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)?;

        file.write_all(contents.as_bytes())?;
        
        println!("Creating config file at: {}", path.display());

        Ok(())

    }
}




#[derive(Debug, Deserialize, Serialize)]
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


