use super::catalog::{self, Artwork};
use anyhow::{Context, bail};
use image::{DynamicImage, ImageReader};
use std::{
    borrow::Cow,
    collections::{BTreeSet, HashMap},
    io::Read,
    path::{Path, PathBuf},
    rc::Rc,
};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

#[derive(Hash, PartialEq, Eq)]
enum ImageKey {
    Bundled(&'static str),
    File(PathBuf),
}

#[derive(Default)]
pub struct Assets {
    decoded: HashMap<ImageKey, Option<Rc<DynamicImage>>>,
    text: HashMap<PathBuf, Option<Rc<str>>>,
    automatic: Option<&'static Artwork>,
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

fn file_path(path: &str, base: &Path) -> anyhow::Result<PathBuf> {
    let path = expand(path, base)?;
    Ok(path.canonicalize().unwrap_or(path))
}

fn read(path: &Path, limit: u64) -> anyhow::Result<Vec<u8>> {
    let file = std::fs::File::open(path)?;
    let metadata = file.metadata()?;
    if !metadata.is_file() {
        bail!("Expected a regular file");
    }

    if metadata.len() > limit {
        bail!("Asset exceeds {} MiB input limit", limit / (1024 * 1024));
    }

    let mut bytes = vec![];
    file.take(limit + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > limit {
        bail!("Asset exceeds {} MiB input limit", limit / (1024 * 1024));
    }
    Ok(bytes)
}

/// Sanitize text art while preserving spacing, blank lines and Unicode display widths.
pub fn ascii(text: &str) -> String {
    let text = rusty_x_ansi::util::strip(text.trim_start_matches('\u{feff}'));
    let text = text.replace("\r\n", "\n");
    let mut result = String::new();
    let mut column = 0;
    for grapheme in text.graphemes(true) {
        match grapheme {
            "\n" => {
                result.push('\n');
                column = 0;
            }
            "\t" => {
                let spaces = 8 - column % 8;
                result.extend(std::iter::repeat_n(' ', spaces));
                column += spaces;
            }
            g if !g.chars().any(char::is_control) => {
                result.push_str(g);
                column += g.width();
            }
            _ => {}
        }
    }
    result
}

impl Assets {
    pub fn load(&mut self, selector: Option<&str>, base: &Path) -> Option<Rc<DynamicImage>> {
        let selector = selector.unwrap_or("auto");
        let bundled = if matches!(selector, "auto" | "builtin") {
            Some(*self.automatic.get_or_insert_with(catalog::automatic))
        } else {
            catalog::find(selector)
        };

        let key = if let Some(art) = bundled {
            ImageKey::Bundled(art.id)
        } else {
            match file_path(selector, base) {
                Ok(path) => ImageKey::File(path),
                Err(e) => {
                    self.warnings.insert(e.to_string());
                    return None;
                }
            }
        };

        if let Some(image) = self.decoded.get(&key) {
            return image.clone();
        }

        let result = (|| -> anyhow::Result<DynamicImage> {
            let bytes = match &key {
                ImageKey::Bundled(id) => Cow::Borrowed(
                    catalog::find(id)
                        .context("Bundled image missing from catalog")?
                        .bytes,
                ),
                ImageKey::File(path) => Cow::Owned(read(path, 64 * 1024 * 1024)?),
            };

            let mut reader =
                ImageReader::new(std::io::Cursor::new(bytes.as_ref())).with_guessed_format()?;
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
                self.warnings.insert(format!(
                    "Image {selector:?}: {e}. Use --list-images for bundled names"
                ));
                None
            }
        };
        self.decoded.insert(key, image.clone());
        image
    }

    pub fn load_ascii(&mut self, path: &str, base: &Path) -> Option<Rc<str>> {
        let path = match file_path(path, base) {
            Ok(path) => path,
            Err(e) => {
                self.warnings.insert(format!("ASCII art {path:?}: {e}"));
                return None;
            }
        };

        if let Some(text) = self.text.get(&path) {
            return text.clone();
        }

        let result = read(&path, 1024 * 1024)
            .and_then(|bytes| String::from_utf8(bytes).context("ASCII art must be UTF-8 text"));
        let text = match result {
            Ok(text) => Some(Rc::<str>::from(ascii(&text))),
            Err(e) => {
                self.warnings
                    .insert(format!("ASCII art {}: {e}", path.display()));
                None
            }
        };
        self.text.insert(path, text.clone());
        text
    }

    #[cfg(test)]
    pub fn decoded_count(&self) -> usize {
        self.decoded.len()
    }
}
