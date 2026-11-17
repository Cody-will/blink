pub mod assets;
pub mod catalog;

use crate::{
    config::Backend,
    layout::{
        node::{Fit, ImageOptions},
        rect::Rect,
    },
    terminal::CellPixels,
};
use anyhow::Context;
use base64::{Engine, engine::general_purpose::STANDARD};
use image::{
    DynamicImage, ImageFormat, RgbaImage,
    imageops::{self, FilterType},
};
use std::io::Write;

pub struct Prepared {
    pub image: DynamicImage,
    pub area: Rect,
}

pub fn prepare(
    image: &DynamicImage,
    area: Rect,
    options: &ImageOptions,
    cell: CellPixels,
) -> Option<Prepared> {
    if area.width == 0 || area.height == 0 {
        return None;
    }

    let pw = u32::from(area.width) * u32::from(cell.width);
    let ph = u32::from(area.height) * u32::from(cell.height);
    if u64::from(pw) * u64::from(ph) > 32 * 1024 * 1024 {
        return None;
    }

    let sx = f64::from(pw) / f64::from(image.width());
    let sy = f64::from(ph) / f64::from(image.height());
    let mut rect = area;
    let output = match options.fit {
        Fit::Stretch => image.resize_exact(pw, ph, FilterType::Triangle),
        Fit::Contain => {
            let scale = sx.min(sy);
            let w = (f64::from(image.width()) * scale).round().max(1.0) as u32;
            let h = (f64::from(image.height()) * scale).round().max(1.0) as u32;
            rect.width = w.div_ceil(u32::from(cell.width)).min(u32::from(area.width)) as u16;
            rect.height = h
                .div_ceil(u32::from(cell.height))
                .min(u32::from(area.height)) as u16;
            rect.x += options.align_h.offset(area.width, rect.width);
            rect.y += options.align_v.offset(area.height, rect.height);
            // Transparent canvas prevents rounding to cell boundaries from stretching the image.
            let mut canvas = RgbaImage::new(
                u32::from(rect.width) * u32::from(cell.width),
                u32::from(rect.height) * u32::from(cell.height),
            );
            let resized = image.resize_exact(w, h, FilterType::Triangle).to_rgba8();
            let x = options.align_h.offset(
                canvas.width().saturating_sub(w).min(u32::from(u16::MAX)) as u16,
                0,
            );
            let y = options.align_v.offset(
                canvas.height().saturating_sub(h).min(u32::from(u16::MAX)) as u16,
                0,
            );
            imageops::overlay(&mut canvas, &resized, i64::from(x), i64::from(y));
            DynamicImage::ImageRgba8(canvas)
        }
        Fit::Cover => {
            let scale = sx.max(sy);
            // Crop the source first to bound allocation for extreme aspect ratios.
            let source_w = (f64::from(pw) / scale)
                .round()
                .max(1.0)
                .min(f64::from(image.width())) as u32;
            let source_h = (f64::from(ph) / scale)
                .round()
                .max(1.0)
                .min(f64::from(image.height())) as u32;
            let offset = |extra: u32, align: crate::styles::Alignment| match align {
                crate::styles::Alignment::Start => 0,
                crate::styles::Alignment::Center => extra / 2,
                crate::styles::Alignment::End => extra,
            };
            image
                .crop_imm(
                    offset(image.width() - source_w, options.align_h),
                    offset(image.height() - source_h, options.align_v),
                    source_w,
                    source_h,
                )
                .resize_exact(pw, ph, FilterType::Triangle)
        }
    };
    Some(Prepared {
        image: output,
        area: rect,
    })
}

pub fn flatten(image: &DynamicImage, background: [u8; 3]) -> DynamicImage {
    let mut pixels = image.to_rgba8();
    for p in pixels.pixels_mut() {
        let a = u32::from(p[3]);
        for i in 0..3 {
            p[i] = ((u32::from(p[i]) * a + u32::from(background[i]) * (255 - a) + 127) / 255) as u8;
        }
        p[3] = 255;
    }
    DynamicImage::ImageRgba8(pixels)
}

pub fn rgb(color: &str) -> [u8; 3] {
    if let Some(hex) = color.strip_prefix('#') {
        let hex = if hex.len() == 3 {
            hex.chars().flat_map(|c| [c, c]).collect::<String>()
        } else {
            hex.into()
        };

        if let Ok(value) = u32::from_str_radix(&hex, 16) {
            return [(value >> 16) as u8, (value >> 8) as u8, value as u8];
        }
    }

    if let Ok(index) = color.parse::<u8>() {
        let basic = [
            [0, 0, 0],
            [128, 0, 0],
            [0, 128, 0],
            [128, 128, 0],
            [0, 0, 128],
            [128, 0, 128],
            [0, 128, 128],
            [192, 192, 192],
            [128, 128, 128],
            [255, 0, 0],
            [0, 255, 0],
            [255, 255, 0],
            [0, 0, 255],
            [255, 0, 255],
            [0, 255, 255],
            [255, 255, 255],
        ];

        if index < 16 {
            return basic[usize::from(index)];
        }

        if index >= 232 {
            return [8 + (index - 232) * 10; 3];
        }

        let n = index - 16;
        let c = |n: u8| if n == 0 { 0 } else { 55 + n * 40 };
        return [c(n / 36), c(n / 6 % 6), c(n % 6)];
    }
    [0, 0, 0]
}

pub trait ImageRenderer {
    fn render(&self, image: &Prepared, output: &mut dyn Write) -> anyhow::Result<()>;
}

pub struct Kitty;
pub struct Iterm2;
pub struct Sixel;
impl ImageRenderer for Kitty {
    fn render(&self, p: &Prepared, output: &mut dyn Write) -> anyhow::Result<()> {
        let rgba = p.image.to_rgba8();
        let encoded = STANDARD.encode(rgba.as_raw());
        let chunks = encoded.as_bytes().chunks(4096);
        let count = chunks.len();
        for (i, chunk) in chunks.enumerate() {
            let more = u8::from(i + 1 < count);
            if i == 0 {
                write!(
                    output,
                    "\x1b_Ga=T,f=32,t=d,s={},v={},c={},r={},C=1,q=2,m={more};",
                    rgba.width(),
                    rgba.height(),
                    p.area.width,
                    p.area.height
                )?;
            } else {
                write!(output, "\x1b_Gm={more};")?;
            }
            output.write_all(chunk)?;
            output.write_all(b"\x1b\\")?;
        }
        Ok(())
    }
}

impl ImageRenderer for Iterm2 {
    fn render(&self, p: &Prepared, output: &mut dyn Write) -> anyhow::Result<()> {
        let mut png = std::io::Cursor::new(Vec::new());
        p.image.write_to(&mut png, ImageFormat::Png)?;
        // Cell/pixel preprocessing already applies the configured fitting mode.
        write!(
            output,
            "\x1b]1337;File=inline=1;width={};height={};preserveAspectRatio=0:{}\x1b\\",
            p.area.width,
            p.area.height,
            STANDARD.encode(png.into_inner())
        )?;
        Ok(())
    }
}

impl ImageRenderer for Sixel {
    fn render(&self, p: &Prepared, output: &mut dyn Write) -> anyhow::Result<()> {
        let rgba = p.image.to_rgba8();
        let sixel = icy_sixel::SixelImage::try_from_rgba(
            rgba.into_raw(),
            p.image.width() as usize,
            p.image.height() as usize,
        )?
        .encode()?;
        output
            .write_all(sixel.as_bytes())
            .context("Writing Sixel image")?;
        Ok(())
    }
}

pub fn native(
    backend: Backend,
    p: &Prepared,
    background: [u8; 3],
    passthrough: bool,
    output: &mut dyn Write,
) -> anyhow::Result<()> {
    let mut bytes = vec![];
    write!(
        &mut bytes,
        "\x1b[{};{}H",
        u32::from(p.area.y) + 1,
        u32::from(p.area.x) + 1
    )?;
    match backend {
        Backend::Kitty => Kitty.render(p, &mut bytes)?,
        Backend::Iterm2 => Iterm2.render(p, &mut bytes)?,
        Backend::Sixel => Sixel.render(
            &Prepared {
                image: flatten(&p.image, background),
                area: p.area,
            },
            &mut bytes,
        )?,
        _ => return Ok(()),
    }

    if passthrough {
        output.write_all(b"\x1bPtmux;")?;
        for b in bytes {
            if b == 0x1b {
                output.write_all(&[b])?;
            }
            output.write_all(&[b])?;
        }
        output.write_all(b"\x1b\\")?;
    } else {
        output.write_all(&bytes)?;
    }
    Ok(())
}

pub fn ansi(p: &Prepared, background: [u8; 3]) -> String {
    let pixels = flatten(&p.image, background)
        .resize_exact(
            u32::from(p.area.width),
            u32::from(p.area.height) * 2,
            FilterType::Triangle,
        )
        .to_rgb8();
    let mut result = String::new();
    for y in 0..p.area.height {
        if y > 0 {
            result.push('\n');
        }

        for x in 0..p.area.width {
            let a = pixels.get_pixel(u32::from(x), u32::from(y) * 2).0;
            let b = pixels.get_pixel(u32::from(x), u32::from(y) * 2 + 1).0;
            let color = |c: [u8; 3]| format!("#{:02x}{:02x}{:02x}", c[0], c[1], c[2]);
            result.push_str(
                &rusty_lipgloss::Style::new()
                    .foreground(&color(a))
                    .background(&color(b))
                    .render("▀"),
            );
        }
    }
    result
}

#[cfg(test)]
mod artwork_tests;
