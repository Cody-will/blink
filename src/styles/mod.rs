use rusty_lipgloss::{self as rl, Style};

use crate::{
    config::theme::Config,
    terminal::Terminal,
};

#[derive(Debug, Clone)]
pub struct ComponentStyle {
    pub border: Style,
    pub label: Style,
    pub content: Style,
}

impl ComponentStyle {
    pub fn new(cfg: &Config) -> Self {
        let palette = &cfg.palette;

        let border = Style::new()
            .border(rl::border::rounded_border(), &[true])
            .border_foreground(&[&palette.border]);

        let label = Style::new()
            .bold(true)
            .foreground(&palette.accent_alt);

        let content = Style::new()
            .foreground(&palette.fg);
             
        Self {
            border,
            label,
            content,
        }
    }
}

pub struct WindowStyle {
    pub style: Style,
}

impl WindowStyle {
    pub fn new(cfg: &Config, terminal: &Terminal) -> Self {
        let palette = &cfg.palette;

        let style = Style::new()
            .width(terminal.width.into())
            .height(terminal.height.into())
            .border(rl::border::rounded_border(), &[true])
            .border_foreground(&[&palette.border]);

        Self { style }
    }
}




