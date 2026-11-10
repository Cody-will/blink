pub mod renderers;
pub mod backend;
pub mod detect;
pub mod img;

use std::io::{self, Write};
use image::DynamicImage;





pub struct ImageRegion {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16
}

pub trait ImageRenderer {
    fn render(&self, image: &DynamicImage, region: &ImageRegion, output: &mut dyn Write) -> io::Result<()>;
}


pub struct KittyRenderer;

pub struct SixelRenderer;

pub struct AnsiRenderer;

pub struct Iterm2Renderer;
