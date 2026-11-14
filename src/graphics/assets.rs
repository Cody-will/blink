use anyhow::{Context, bail};
use image::{DynamicImage, ImageReader};
use std::{
    collections::{BTreeSet, HashMap},
    path::{Path, PathBuf},
    rc::Rc,
};
#[derive(Default)]
pub struct Assets {
    decoded: HashMap<PathBuf, Option<Rc<DynamicImage>>>,
    pub warnings: BTreeSet<String>,
}
pub fn expand(path: &str, base: &Path) -> anyhow::Result<PathBuf> {
    if path == "~" || path.starts_with("~/") || path.starts_with("~\\") {
        return Ok(dirs::home_dir()
            .context("Home directory unavailable")?
            .join(path.get(2..).unwrap_or("")));
    }
    if path.starts_with('~') {
        bail!("Only ~ and ~/ home paths are supported");
    }
    Ok(base.join(path))
}
impl Assets {
    pub fn load(&mut self, path: Option<&str>, base: &Path) -> Option<Rc<DynamicImage>> {
        let path = match path.map(|p| expand(p, base)).transpose() {
            Ok(p) => p.unwrap_or_else(|| PathBuf::from("<bundled-astronaut>")),
            Err(e) => {
                self.warnings.insert(e.to_string());
                return None;
            }
        };
        let path = path.canonicalize().unwrap_or(path);
        if let Some(image) = self.decoded.get(&path) {
            return image.clone();
        }
        let result = (|| -> anyhow::Result<DynamicImage> {
            let bytes = if path == Path::new("<bundled-astronaut>") {
                include_bytes!("../../assets/Pixel Astronaut Among Planets-3.png").to_vec()
            } else {
                if std::fs::metadata(&path)?.len() > 64 * 1024 * 1024 {
                    bail!("Image exceeds 64 MiB input limit");
                }
                std::fs::read(&path)?
            };
            let mut reader = ImageReader::new(std::io::Cursor::new(bytes)).with_guessed_format()?;
            let mut limits = image::Limits::default();
            limits.max_image_width = Some(16384);
            limits.max_image_height = Some(16384);
            limits.max_alloc = Some(128 * 1024 * 1024);
            reader.limits(limits);
            Ok(reader.decode()?)
        })();
        let image = match result {
            Ok(image) => Some(Rc::new(image)),
            Err(e) => {
                self.warnings
                    .insert(format!("Image {}: {e}", path.display()));
                None
            }
        };
        self.decoded.insert(path, image.clone());
        image
    }
    #[cfg(test)]
    pub fn decoded_count(&self) -> usize {
        self.decoded.len()
    }
}
