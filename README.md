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
library (for example `libxcb1-dev` on Debian/Ubuntu or `libxcb` on Arch),
GLib/GIO development files (`libglib2.0-dev` on Debian/Ubuntu or `glib2` on Arch),
plus a C compiler and `pkg-config`. Desktop/display information is optional at runtime;
headless systems still work. Sixel encoding is pure Rust and needs no libsixel.
All 30 bundled images are embedded in the executable, including installed builds.
Original artwork is preserved; the image collection adds about 84 MiB to the binary.

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
| `--list-images` | List bundled image names, descriptions, and OS defaults |
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
and checking do not create files or collect the system metric snapshot. Asset
checking resolves automatic artwork for the current OS.

The generated default is 34 lines and includes all 29 entries from Fastfetch
2.71's default structure, plus Blink's hostname and core-count rows. Unavailable
information is hidden by default. The main settings look like this:

```toml
theme = "gruvbox-material"
background = "transparent"

[layout]
direction = "horizontal"

[layout.first]
image = "auto"
size = "35%"

[layout.second]
responsive = true
header = ["title", "separator"]
footer = ["break", "colors"]
modules = [
    { title = "System", components = ["os", "host", "hostname", "kernel", "uptime", "packages", "shell", "locale"] },
    { title = "Session", components = ["display", "desktop", "window_manager", "wm_theme", "desktop_theme", "icon_theme", "font", "cursor", "terminal", "terminal_font"] },
    { title = "Hardware", components = ["cpu", "cpu_cores", "gpu", "memory", "swap"] },
    { title = "Storage & Power", components = ["disk", "local_ip", "battery", "power_adapter"] },
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
| `ascii` | Inline `content` or a UTF-8 art file `path` |
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

Nodes accept `visible = false`. With `responsive = true` on a column section,
modules pack into one to three columns, each at least 28 cells wide. Header and
footer components span the section. Modules go into the shortest column, with
configuration order breaking ties. If the dashboard is too tall, it removes
optional artwork, then palette/decorations, appearance details, and lower-priority
rows. OS, CPU, and memory stay longest; extremely small rectangles still clip.
Plain output keeps every available requested metric regardless of terminal size.
Use `responsive = false` to keep a fixed stack.

Existing explicit module/children layouts remain fixed unless they opt in.
In fixed layouts, `optional = true` permits the existing small-terminal trimming:
images below 64 columns or 20 available rows; swatches below 64 columns or 24 rows;
optional components below 40 columns or 14 rows. Presets mark hostname, uptime,
shell, and core-count rows optional. Overflow clips safely. Removing an image
section lets the other section consume the full available area.

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

Default data components:

`os`, `host`, `hostname`, `kernel`, `uptime`, `packages`, `shell`, `display`,
`desktop`, `window_manager`, `wm_theme`, `desktop_theme`, `icon_theme`, `font`,
`cursor`, `terminal`, `terminal_font`, `cpu`, `cpu_cores`, `gpu`, `memory`, `swap`,
`disk`, `local_ip`, `battery`, `power_adapter`, and `locale`.

Additional data components: `distribution`, `architecture`, `cpu_usage`,
`total_ram`, `used_ram`, `memory_percentage`, and `datetime`. Presentation
components are `title` (`user@host`), `separator` (title display width), `break`,
and `colors` (16 ANSI colors; omitted from plain/no-color output). The existing
`swatches` node renders theme colors independently.

Names are case-insensitive. Fastfetch aliases `DE`, `WM`, `WMTheme`, `Theme`,
`Icons`, `TerminalFont`, `LocalIp`, and `PowerAdapter` are accepted and normalized.
`desktop_theme` reports the desktop's appearance; top-level `theme` chooses
Blink's own palette. [Component coverage and native sources](docs/components.md)
describe detection limits, including supported package databases and terminal fonts.

Collection is driven by visible requested components, with one shared snapshot.
CPU usage takes two samples at sysinfo's minimum interval and is absent from the
default dashboard. Other components do not incur that delay. Memory/storage use
binary units; percentages require a valid nonzero total. Zero swap is reported as
`Not configured (0 B)`. Local IP lists operational, non-loopback IPv4/IPv6 interfaces
and makes no external network requests. Power adapters report connection state and
measured watts only when available; measured draw is not charger rated capacity.

Unavailable information is hidden. Set `[display] unavailable = "show"` to display
`N/A` instead. Shell detection prefers a recognized Linux parent shell, then
`SHELL`/`COMSPEC`. Desktop/window-manager evidence comes from session data.
Remote/headless sessions and some Wayland compositors may not expose physical
displays. GPU availability depends on sysinfo's drivers/APIs. No collector launches
a shell command, and Fastfetch is not required to run Blink.

Unicode symbols are the default; font detection is not reliable. Select
`[icons] mode = "ascii"` for safe glyphs or `"nerd"` when you have a Nerd Font.
Set `enabled = false`, use `--no-icons`, or override a component with `icon = "◆"`.
Raster files use `icon_path = "icons/cpu.png"`; arbitrary paths are never printed
as glyphs. SVG icons are not supported. Raster icons share the image backend and
are replaced by normal component text in plain output.

## Images and terminal support

The default `image = "auto"` selects variant 1 for the current OS/distribution.
Unknown distributions and unsupported systems use `blink-1`; parent-distribution
artwork is not inferred. `builtin` and an omitted image source also mean automatic
selection. Omarchy uses `omarchy-1`, including older Arch-based installations
with identifiable Omarchy runtime files. Variants `omarchy-2` and `omarchy-3`
are selectable too. Any bundled image can be selected on any OS:

```toml
[layout.first]
image = "linuxmint-2"  # also accepts "linuxmint-2.png"
```

Run `blink --list-images` for the complete catalog. The [artwork guide](docs/artwork.md)
includes descriptions and the original-to-new filename mapping. The astronaut is
now `arch-2`, because its shoulder patch carries the Arch logo. Original bare
filenames remain accepted as catalog aliases.

Custom image paths expand `~` and resolve relative to the configuration directory:

```toml
[layout.first]
image = "~/Pictures/my-logo.png"
```

Recognized bare names select bundled assets. Include a directory or `./` to select
a custom file with the same name: `image = "./arch-1.png"`. PNG, JPEG, WebP, and
GIF (first frame) are supported. Selected assets are decoded once per invocation.
Input/decode limits bound memory use. Missing/corrupt images warn once on stderr
and use a placeholder; `--check-config` treats them as validation errors.

ASCII art uses the same layout, alignment, clipping, and text styles:

```toml
[layout.first]
ascii = "~/art/logo.txt"
# Alternatively: ascii = { text = "..." }
```

For multiline inline art, use a TOML literal string:

```toml
[layout.first]
ascii = { text = '''
  /\_/\
 ( o.o )
  > ^ <
''' }

[layout.first.style]
align_h = "center"
align_v = "center"

[layout.first.text_style]
foreground = "@accent"
```

Choose one source per node. Detailed ASCII nodes use `type = "ascii"` and either
`path` or `content`. Files must be UTF-8 and at most 1 MiB. Spaces and blank lines
are retained, CRLF is normalized, tabs use eight-cell stops, and terminal escape
sequences are removed; apply colors through Blink styles instead. ASCII stays in
plain output and with `--no-image`. Mark it `optional = true` to permit removal
when a responsive dashboard needs the space. Missing/invalid files warn once and
show `[art unavailable]`; `--check-config` reports an error. Built-in ASCII logos
and raster-to-ASCII conversion are not implemented.

Raster fitting applies to image nodes:

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
- [Custom inline ASCII artwork](examples/ascii.toml)
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
