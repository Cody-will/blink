mod config;
mod layout;
mod styles;
mod render;
mod terminal;
mod graphics;
mod components;

use crate::layout::engine::LayoutEngine;
use crate::layout::rect::Rect;
use crate::config::theme::Config;
use crate::terminal::Terminal;


pub const DEFAULT_IMG_PATH: &str = "/home/cody/Code/blink/assets/Pixel Astronaut Among Planets-3.png";

fn main() -> anyhow::Result<()>{
    
    let cfg = Config::load()?;
    let term = Terminal::detect()?;

    let window_area = Rect{x: 0, y: 0, width: term.width, height: term.height};

    let content_area = window_area.inner(&cfg.window.container);

    let render_nodes = LayoutEngine::calculate(&cfg.layout, content_area);
    
    // let path = Path::new("/home/cody/Code/blink/assets/Pixel Astronaut Among Planets-3.png");
    // let load = graphics::renderers::kitty::load_image(path)?;
    // let (width, height) = (load.width(), load.height());
    // let (cols, rows) = (width / 50, height / 50);
    // let converted = graphics::img::to_rgba(&load);
    // let img = graphics::renderers::kitty::render_kitty(&converted, cols as u16, rows as u16);
    //
    // render(&cfg, &term)?;
    //
    
    println!("{:#?}", render_nodes);

    Ok(())
}
