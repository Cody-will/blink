# Changelog

## Unreleased — planned 0.1.0

- One-shot system dashboard with horizontal/vertical splits and nested layouts.
- Compact TOML configuration, transparent defaults, themes, and aligned components.
- Responsive modules and expanded system/session/hardware/storage information.
- 30 embedded images with OS selection, including Omarchy, plus custom raster and ASCII art.
- Kitty, iTerm2, Sixel, ANSI, and text image output; clean plain/redirected output.

### Compatibility and limitations

Existing configurations are preserved. `image = "builtin"` selects OS artwork;
use `arch-2` to retain the former astronaut. Linux is the primary tested platform.
Native graphics and other operating systems require platform-specific validation.
The Linux executable uses shared system libraries; consult packaged BUILD-INFO.txt.
