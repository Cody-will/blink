mod config;
mod layout;
mod styles;
mod render;
mod terminal;
mod graphics;
mod components;
use image::GenericImageView;

use crate::render::render;
use crate::config::theme::Config;
use crate::terminal::Terminal;
use std::path::Path;

fn main() -> anyhow::Result<()>{
    
    let cfg = Config::load()?;
    let term = Terminal::detect()?;
    
    let path = Path::new("/home/cody/Code/blink/assets/Pixel Astronaut Among Planets-3.png");
    let load = graphics::renderers::kitty::load_image(path)?;
    let (width, height) = (load.width(), load.height());
    let (cols, rows) = (width / 50, height / 50);
    let converted = graphics::img::to_rgba(&load);
    let img = graphics::renderers::kitty::render_kitty(&converted, cols as u16, rows as u16);

    //render(&cfg, &term)?;

    Ok(())
}
