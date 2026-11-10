use image::{DynamicImage, RgbaImage};
use base64::{engine::general_purpose::STANDARD, Engine};


pub fn to_rgba(img: &DynamicImage) -> RgbaImage {
    img.to_rgba8()  
}


