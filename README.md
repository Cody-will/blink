# Blink

Blink is a one-shot Rust system-information dashboard built with rusty_lipgloss.
It fills the available terminal area, leaves space for your shell prompt, and
exits. Layouts, modules, components, images, icons, and styles are configured in
TOML; there is no event loop, alternate screen, or background service.
Dashboard output clears the visible terminal screen before drawing. Plain and
redirected output never clear the screen.

## Build and install

Use a current stable Rust toolchain supporting edition 2024 and let chains.

```sh
cargo build --release
cargo install --path . --locked
blink
```

On Linux, building the display-information dependency requires an XCB development
library (for example `libxcb1-dev` on Debian/Ubuntu or `libxcb` on Arch), plus a C
compiler and `pkg-config`. Desktop/display information is optional at runtime;
headless systems still work. Sixel encoding is pure Rust and needs no libsixel.
The bundled astronaut is embedded in the executable, including installed builds.

Linux is the primary tested platform. Collectors use portable libraries and
fallbacks for macOS and Windows, but those platforms have not been runtime-tested
here. This project does not claim full platform or terminal visual certification.

## Usage

```sh
blink --config examples/default.toml
blink --theme nord --layout vertical
blink --no-image --no-icons
blink --plain > system.txt
blink --check-config
blink --print-config > blink-config.toml
```

| Option | Behavior |
| --- | --- |
| `--help`, `--version` | CLI help and application version |
| `--config PATH` | Select a TOML file; normal launches create it if missing |
| `--theme NAME` | Override the theme for this invocation |
| `--layout horizontal\|vertical` | Override the top-level split |
| `--graphics auto\|kitty\|iterm2\|sixel\|ansi\|text` | Select image output, subject to output/multiplexer safety rules |
| `--no-image` | Remove image nodes and raster icons before layout |
| `--no-icons` | Disable component glyph and file icons |
| `--no-color` | Disable styling escapes and use text image placeholders |
| `--plain` | Print readable titles and aligned text rows, without dashboard geometry |
| `--print-config` | Print concise, editable TOML, preserving custom settings and CLI overrides |
| `--list-themes` | List built-in theme names |
| `--list-modules` | List module presets and all system component names |
| `--check-config` | Validate configuration, colors, and configured visible assets |
| `--width N`, `--height N` | Override terminal dimensions for previews |

Redirected output is always plain, even with an explicit graphics override. It
has no cursor controls, SGR styles, native images, or terminal-height padding.
`NO_COLOR` disables colors in the dashboard; `TERM=dumb` selects plain output.
`--no-color` retains cursor positioning; use `--plain` to remove every terminal
control sequence.

## Configuration and defaults

The platform configuration directory contains `blink/config.toml`
(`~/.config/blink/config.toml` on Linux). Normal first launch creates a compact, editable
config with `create_new`, never overwriting an existing file. Listing, printing,
and checking do not create files or collect system information.

The generated default is 23 lines. It exposes the settings most people edit:

```toml
theme = "gruvbox-material"
background = "transparent"

[layout]
direction = "horizontal"

[layout.first]
image = "builtin"
size = "35%"

[layout.second]
swatches = true
modules = [
    { title = "System", components = ["os", "hostname", "kernel", "uptime", "shell"] },
    { title = "Hardware", components = ["cpu", "cpu_cores", "memory"] },
]
```

Transparency is the default, so `background` can be omitted. To use a solid
background, set a color or palette reference such as `background = "@bg"`.
All other omitted options use built-in defaults. You do not need to copy empty
tables, image settings onto text components, or every internal field.

Change a component's options by replacing its name with a small table:

```toml
[[layout.second.modules]]
title = "Custom"
components = [
    "os",
    { name = "memory", label = "RAM", icon = "M" },
    { label = "Profile", value = "Development" },
]
```

Use either the inline `modules = [...]` form or `[[layout.second.modules]]`
tables for a given section. Module strings such as `modules = ["system", "storage"]`
select presets. `direction = "horizontal"` arranges a section's modules side by
side; the default is vertical. Horizontal modules share space by default;
vertical modules fit their content. Explicit sizes override that behavior.
`modules = []` empties a section and `components = []` empties a module.
Component-name shortcuts give rows automatic sizing and the same optional-row
policy as presets; `{ name = "hostname", optional = false }` keeps an optional
row visible on small terminals.

Align the children of a section with one setting:

```toml
[layout.second]
align = "center"
```

`align` sets both axes. Use `align_h` and `align_v` independently when needed:

```toml
[layout.second]
align_h = "start"
align_v = "center"
```

Values are `start`, `center`, and `end`: left/center/right horizontally and
top/center/bottom vertically. Along the layout direction, the modules move as a
group, retaining their gaps. On the other axis, each module fits its content and
is positioned independently. Omitted alignment keeps the existing full-width
behavior for vertical stacks (full-height for horizontal rows).

Alignment positions module containers; their own text styles remain independent.
A `fill` size consumes spare space along the layout direction, leaving no room
to shift that group on that axis. For horizontally centered content-sized
modules, set their `size = "auto"`. Alignment also works on nested rows, columns,
and modules using the detailed syntax.

The detailed recursive `type`/`children` format below remains available for
mixed content and advanced layouts. Both forms normalize to the same layout
engine. Avoid specifying both `modules` and `children` (or both `components` and
`children`) on the same node. Styles still merge field by field, and explicit
colors remain explicit when changing themes.

Existing long files keep working and are never automatically overwritten.
Export a compact equivalent without losing custom settings:

```sh
blink --print-config > ~/.config/blink/config.compact.toml
blink --config ~/.config/blink/config.compact.toml --check-config
blink --config ~/.config/blink/config.compact.toml
```

Keep the new file beside the original so relative image paths have the same
base directory. The exporter verifies that reading its output preserves the
configuration before returning it.

The current earlier-prototype format remains readable. Original `window`,
`module`, `component`, and tagged split tables are converted in memory where
unambiguous. Unmappable settings, including differing legacy section styles or a
nonzero global module gap, produce an actionable error rather than silently
lose settings. Legacy icon-path nodes must be changed to image nodes or
`icon_path` fields. Export normalized TOML to a *new* file with `--print-config`
and review it before replacing your original configuration.

## Layouts and modules

`layout.first` and `layout.second` are left/right for a horizontal split and
top/bottom for a vertical split. Either can contain any supported node:

| `type` | Fields and content |
| --- | --- |
| `row` / `column` | Recursive `children`, with a configurable `gap` |
| `module` | Optional `title`, `title_align`, `direction`, and `children` or `preset` |
| `component` | Dynamic `name` or static `value`, optional `label`, `icon`, `icon_path` |
| `text` | Literal multiline `content` |
| `image` | Optional `path` and `image` fitting options |
| `swatches` | Compact colors from the current palette |

`preset = "system"`, `"hardware"`, `"storage"`, or `"session"` expands to a module
component list. Use `children` for custom lists; do not combine it with a preset.
Within a container, direct component children share display-cell icon/label
column widths automatically. Values may span multiple aligned rows.

```toml
[layout.second]
type = "row"
gap = 1
children = [
    { type = "module", preset = "system", title_align = "start" },
    { type = "module", title = "Custom", children = [
        { type = "component", name = "cpu", size = "auto" },
        { type = "component", value = "Development", label = "Profile", size = "auto" },
        { type = "text", content = "Hello\nworld", size = "auto" },
    ] },
]
```

Node `size` accepts `"fill"`, `"auto"`, an integer such as `12`, or a
percentage such as `"35%"`. The older `{ fixed = 12 }` and `{ percent = 35 }`
forms still work. It applies along the parent's layout axis. Requests are
allocated in configuration order, bounded to available space after gaps; fill
children divide the remainder. Auto measures content, borders, and padding.
Borders and padding consume space *inside* the assigned rectangle.

Nodes accept `visible = false`. `optional = true` permits small-terminal trimming:
images are omitted below 64 columns or 20 available rows; swatches below 64
columns or 24 available rows; optional components below 40 columns or 14 available
rows. The default presets mark host, uptime, shell, and core-count rows optional.
Custom layouts never automatically switch orientation; overflow clips safely.
Removing an image section lets the other section consume the full available area.

`reserved_rows` defaults to two; at least one row is always left for the returning
prompt. Match this setting to your shell prompt, whose height cannot reliably be
detected. Terminal dimensions normally already exclude the terminal's own UI.
Dimensions default to 80×24 only when terminal queries fail. Resource limits reject
areas above one million cells instead of attempting unbounded allocation.

## Themes and styling

Themes: `gruvbox-material` (default), `gruvbox-dark`, `catppuccin-mocha`,
`tokyo-night`, `nord`, `dracula`, and `monochrome`. `gruvbox` is a compatibility
alias for `gruvbox-dark`.

The palette has `fg`, `bg`, `muted`, `border`, `accent`, `accent_alt`, `success`,
`warning`, and `error`. Override individual values:

```toml
[palette]
accent = "#fabd2f"

[styles.window]
background = "transparent"

[styles.module.border]
kind = "rounded"
color = "@border"

[styles.label]
foreground = "@accent"
bold = true
```

Container styles: `styles.window`, `styles.section`, `styles.module`, and
`styles.component`. They support `background`, `border.kind`, `border.color`,
`padding.top/bottom/left/right`, and `align_h/align_v` (`start`, `center`, `end`).
Border kinds are `none`, `normal`, `rounded`, `thick`, `double`, and `hidden`.
Hidden reserves border space; none does not.

Text styles: `styles.text`, `styles.title`, `styles.label`, and `styles.icon`.
They support `foreground`, optional `background`, `bold`, `italic`, and
`underline`. Colors accept `#RGB`, `#RRGGBB`, string indices `"0"`–`"255"`, or
`@palette` references. Unknown references and invalid colors are errors.

Set `background = "transparent"` on any container or text style to use the
terminal's default background, including its configured transparency. An omitted
background inherits its container; explicit `"transparent"` clears that inheritance.
The dashboard is transparent by default. `background = "transparent"` at the
top level sets the window background; `[styles.window]` remains available for
advanced overrides. Specify the window background in only one place.

Each node has partial `style`, `text_style`, `label_style`, and `icon_style`
overrides. For example:

```toml
[layout.second.style.padding]
left = 2

# Inline component form:
# { type = "component", name = "cpu", label_style = { italic = true } }
```

## Components and icons

Available component names:

`os`, `distribution`, `hostname`, `kernel`, `architecture`, `uptime`, `shell`,
`terminal`, `cpu`, `cpu_usage`, `cpu_cores`, `memory`, `total_ram`, `used_ram`,
`memory_percentage`, `gpu`, `disk`, `battery`, `desktop`, `window_manager`,
`display`, and `datetime`.

Collection is driven by visible requested components, with one shared snapshot.
CPU usage takes two samples at sysinfo's minimum interval and is absent from the
default dashboard. Other components do not incur that delay. Memory/storage use
binary units; percentages require a valid nonzero total. Disk output lists mount
capacity and used space; batteries include charge percentage and charging state.
Missing information displays `N/A`; `[display] unavailable = "hide"` omits it.

Shell detection prefers a recognized Linux parent shell, then `SHELL`/`COMSPEC`.
Terminal/desktop names come from session environment data. Window manager names
are shown only with explicit session evidence. Remote/headless sessions and some
Wayland compositors may not expose physical display information. GPU availability
depends on the drivers and APIs supported by sysinfo. No collector launches a
shell command to manufacture a value.

Unicode symbols are the default; font detection is not reliable. Select
`[icons] mode = "ascii"` for safe glyphs or `"nerd"` when you have a Nerd Font.
Set `enabled = false`, use `--no-icons`, or override a component with `icon = "◆"`.
Raster files use `icon_path = "icons/cpu.png"`; arbitrary paths are never printed
as glyphs. SVG icons are not supported. Raster icons share the image backend and
are replaced by normal component text in plain output.

## Images and terminal support

Image paths expand `~` and resolve relative to the configuration directory.
Omit `path` to use the embedded astronaut. PNG, JPEG, WebP, and GIF (first frame)
are supported. Assets are decoded once per invocation. Input and decode limits
bound memory use. Missing/corrupt images warn once on stderr and use a placeholder;
`--check-config` treats them as validation errors.

```toml
[layout.first.image]
fit = "contain"  # cover crops; stretch fills without preserving aspect ratio
align_h = "center"
align_v = "center"
```

Fitting uses reported cell pixel dimensions where available; otherwise
`[display] cell_width = 8` and `cell_height = 16` provide estimates. Contain
preserves aspect ratio and transparency; cover crops within the assigned region.
ANSI and Sixel composite partial transparency against the resolved background.

Automatic priority is Kitty → iTerm2 → Sixel → ANSI half-blocks → text. Native
support comes from capability reports, not terminal-name guesses. Unix probes
have a configurable `probe_timeout_ms = 50` deadline and restore terminal modes
on return/error. Set `[display] probe = false` to avoid queries or use an explicit
backend. Unsupported or slow responses fall back to ANSI. On Windows, automatic
selection uses advertised features; force the backend when necessary.

Native protocol encoding and placement are covered by automated/PTY tests.
Kitty, iTerm2, and Sixel have **not** been visually verified in real emulator
windows in this environment. Their actual behavior, including terminal-specific
image scrolling, remains a release validation task.

Multiplexers use ANSI unless `[display] multiplexer_passthrough = true`. Tmux
passthrough requires terminal/multiplexer support and suitable tmux settings;
image positioning in split panes is not certified. GNU screen passthrough is not
implemented. Use ANSI there. Explicit overrides cannot bypass the plain-output
policy or the default multiplexer safety policy.

## Examples and troubleshooting

- [Horizontal dashboard](examples/default.toml)
- [Vertical image and modules](examples/vertical.toml)
- [Minimal text-only display](examples/minimal.toml)
- [Compact small-terminal display](examples/compact.toml)
- [Rich bordered dashboard](examples/dashboard.toml)

If glyphs show as boxes, choose ASCII icons. If native images are missing, try
`--graphics ansi`, inspect `--check-config`, or disable probes. If text is clipped,
remove optional rows, reduce padding, or adjust section sizes. Long lines clip
rather than wrap or change the configured orientation. If a slow link leaves
late capability responses, disable probing or increase its timeout. Error messages
go to stderr; normal rendering does not print configuration-loading notices.

## Development

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo build --release
```

See [the implementation and validation report](docs/implementation.md) for
architecture, tests, performance measurements, and remaining validation work.
