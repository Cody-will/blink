use image::{DynamicImage, ImageReader, RgbaImage};
use std::path::Path;
use std::io::{self, Write};
use base64::{engine::general_purpose::STANDARD, Engine};

pub fn load_image(path: &Path) -> anyhow::Result<DynamicImage> {
    let image = ImageReader::open(path)?
        .decode()?;
    Ok(image)
}


pub fn render_kitty(
    img: &RgbaImage,
    columns: u16,
    rows: u16,
) -> io::Result<()> {
    let encoded = STANDARD.encode(img.as_raw());

    let width = img.width();
    let height = img.height();

    let stdout = io::stdout();
    let mut output = stdout.lock();

    let chunks: Vec<&[u8]> = encoded
        .as_bytes()
        .chunks(4096)
        .collect();

    for (index, chunk) in chunks.iter().enumerate() {
        let more = if index + 1 < chunks.len() {
            1
        } else {
            0
        };

        if index == 0 {
            write!(
                output,
                "\x1b_Ga=T,f=32,t=d,s={width},v={height},c={columns},r={rows},q=2,m={more};"
            )?;
        } else {
            write!(output, "\x1b_Gm={more};")?;
        }

        output.write_all(chunk)?;
        output.write_all(b"\x1b\\")?;
    }

    output.flush()?;

    Ok(())
}
