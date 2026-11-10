use crossterm::terminal;

use crate::layout::rect::Rect;


#[derive(Debug, Clone, Copy)]
pub struct Terminal {
    pub width: u16,
    pub height: u16,
}

impl Terminal {
    pub fn detect() -> anyhow::Result<Self> {
        let (width, height) = terminal::size()?;

        Ok(Self {width, height})
    }

    pub fn area(&self) -> Rect {
        Rect { x: 0, y: 0, width: self.width, height: self.height }
    } 
}
