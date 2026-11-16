# Implementation and validation

## Architecture

The existing recursive layout engine, rectangle geometry, Lip Gloss styling,
image encoders, and embedded artwork remain in use. Configuration is separate
from system collection and terminal I/O. The generated default is 34 lines,
with transparent backgrounds, OS-selected artwork, and four editable module lists. Detailed recursive
layouts and existing files remain supported; no automatic on-disk migration occurs.

1. `config` loads and normalizes TOML, expands presets/aliases, validates settings,
   and converts recognizable legacy layouts. Concise export verifies a lossless
   read-back before returning its document.
2. `terminal` captures dimensions/cell pixels and bounded graphics capabilities.
3. `components::system` collects requested measurements into one typed snapshot.
   `components::native` reads package databases, host hardware, interfaces, adapter
   state, locale, and appearance settings. The registry owns labels, icons, module
   lists, aliases, presentation kinds, and trimming priorities.
4. `content` resolves availability, assets, styles, presentation rows, and aligned
   component columns. `measurement` computes natural display-cell dimensions.
5. `responsive` packs only opted-in column sections into one to three lanes, with
   header/footer groups spanning the section. It operates on resolved content,
   without changing the source configuration or recollecting data. If height is
   insufficient, optional artwork and then lower-priority rows are omitted.
6. `layout::engine` allocates bounded recursive rectangles and retains styled
   containers. `styles` builds Lip Gloss styles; `render` composes them on its
   canvas, positions native images, and restores the prompt cursor. Plain output
   skips responsive trimming and dashboard geometry.
7. `graphics` handles fitting, alpha compositing, asset reuse, and protocol encoding.
   Its catalog embeds 30 images under stable names, preserving the original files
   and bare-filename aliases. Automatic selection matches the native OS/distribution
   once per invocation, always variant 1, with `blink-1` as the unknown-system fallback.
   ASCII file/inline sources resolve to styled text and preserve art whitespace.

The registry covers every entry in the Fastfetch 2.71 default structure, with a
checked fixture preventing accidental omissions. This does not claim every optional
Fastfetch module or equivalent platform detection. [Component coverage](components.md)
lists sources and limitations. Native detection never invokes external commands.

Main additions for this expansion: `src/components/native/{mod,packages,appearance,tests}.rs`,
`src/responsive.rs`, the default-list fixture, expanded registry/config/content
handling, responsive/native tests, and the compact default example. The generic
layout engine and graphics implementations did not require a replacement.

## Behavior and limits

The dashboard uses the terminal area minus configured prompt rows, reserving at
least one. The default reserves two. Small responsive sections drop lower-priority
rows while retaining OS, CPU, and memory longest; extremely small terminals clip
safely. Explicit custom module/children layouts retain their fixed arrangement
unless `responsive = true` is configured. Plain output keeps all available requested
metrics. Missing values are hidden by default; `unavailable = "show"` displays `N/A`.
CPU usage is opt-in because it requires an additional sampling interval.

New Linux package sources cover pacman, dpkg, RPM SQLite, Flatpak, and Snap. GNOME
uses GSettings directly; GTK/KDE/Xfce/Openbox and supported terminal fonts use
native settings/configuration. Dynamic terminal overrides and unsupported sources
remain unavailable. Adapter wattage describes measured draw, not rated capacity.
Linux is the runtime-tested OS; macOS/Windows builds and native detection need
external validation. Existing portable metrics retain their native libraries.

Themes use palette references; explicit user colors remain explicit. Ambiguous
legacy mappings fail with actionable errors. Existing files are never rewritten,
including older compact configurations, so adopting the expanded defaults requires
choosing the new example or editing/exporting a new configuration file.

Kitty, iTerm2, and Sixel encoders retain automated protocol coverage, with ANSI/text
fallbacks. Native emulator windows were unavailable for visual certification.
Sixel colors are quantized; ANSI/Sixel flatten transparency onto the resolved
background. SVG, animated playback, GNU screen passthrough, and certified tmux
split-pane image positioning remain outside current support.

## Validation

The following passed on Linux after this expansion:

- `cargo fmt --check`
- `cargo clippy --offline --all-targets -- -D warnings`
- `cargo test --offline`: 85 unit tests and 13 CLI integration tests
- `cargo build --offline --release`
- `cargo install --offline --path . --locked --root /tmp/blink-artwork-installed`
- Installed binary run from `/tmp`, including all 30 bundled selections, plain output,
  ASCII art, and embedded-asset validation.

Tests cover the complete default-list mapping, aliases and defaults, compact and
legacy config preservation, missing/empty/malformed/unreadable native sources,
read-only RPM SQLite, package counting policies, active desktop settings, terminal
font includes, adapter units/state, IP filtering, zero swap, presentation display
widths and colors, responsive packing/priorities, both orientations, nested layout
bounds, custom layout stability, aligned multiline values, and complete plain output.
Artwork tests cover all 30 decodes and aliases, exact OS mapping/fallback, custom-file
precedence, lossless config export, ASCII styling/whitespace/tabs/Unicode, invalid
UTF-8 and bounded reads, optional removal, plain output, and catalog CLI inspection.
Rename SHA-256 verification confirms the original image bytes are unchanged.
Existing theme, image/protocol, capability, geometry, CLI, and broken-pipe tests pass.

Release pseudo-terminal checks covered horizontal and vertical layouts at 80×24,
120×40, 160×60, 32×12, 3×4, and 1×1; ANSI artwork at 160×60; no-color output;
and automatic probe timeout fallback. ASCII artwork was also checked in both
orientations at five dimensions from 160×60 to 1×1. Emitted text/cursor positions stayed within
terminal bounds; normal output cleared the screen and left prompt space. Original
termios attributes were preserved. These are byte/state checks, not native-emulator
visual certification.

The following measurements predate the OS-aware artwork catalog and use the former
astronaut image; they are retained as historical measurements, not current artwork
performance claims. Release timings on this Linux machine, warm filesystem caches, ten subprocess
runs per scenario, including startup and output consumption:

| Scenario | Median | Minimum |
| --- | ---: | ---: |
| 80×24 responsive dashboard (art omitted to fit) | 61.08 ms | 59.19 ms |
| 160×60 dashboard with ANSI artwork | 114.60 ms | 111.84 ms |
| 80×24 dashboard with `--no-image` | 43.67 ms | 39.53 ms |
| 160×60 auto backend, unanswered 50 ms probe | 179.37 ms | 164.78 ms |
| Complete default, plain output | 30.52 ms | 24.20 ms |

These expanded-default measurements are machine/session-specific, not portable
latency guarantees. PTY polling resolution was 5 ms; image size, mounted disks,
package databases, and desktop settings affect timing. Probes run only if images
remain after layout. Use an explicit backend or disable probes to avoid that
wait, and omit CPU usage when its sampling delay is undesirable.

## Running the expanded default

```sh
cargo run --release -- --config examples/default.toml
cargo run --release -- --config examples/default.toml --plain
cargo run --release -- --config examples/default.toml --layout vertical
```

A missing normal config is generated on first launch. Existing configs keep their
chosen component lists. To adopt the new default without replacing your current
file, copy `examples/default.toml` to a new file and select it with `--config`.
See the README for configuration paths, module customization, graphics, and styles.

## Future work

- Visually validate native graphics in Kitty, iTerm2, and a Sixel emulator,
  including multiple images, transparency, scrolling, and tmux panes.
- Add macOS/Windows build/runtime CI and native collector fixtures.
- Expand package/appearance detection only where reliable native sources exist.
- Add live terminal-font/profile detection where a native interface is available.

The [artwork guide](artwork.md) lists all renamed images and compatibility behavior.
Original images add about 84 MiB to the executable; only selected images are decoded.
The verified Linux release binary is approximately 92 MiB.
