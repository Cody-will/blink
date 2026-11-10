

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphicsBackend {
    Kitty,
    Sixel,
    Iterm2,
    Ansi,
    Ascii,
}
