# Implementation and validation

## Architecture

The existing recursive layout and saturating rectangle geometry remain the core.
Configuration describes content; it does not perform collection or terminal I/O.
The normal config file is a compact user-facing document, not a serialized render
tree. Image/module/component shortcuts normalize into the existing recursive model.
First launch and `--print-config` emit concise TOML; the exporter verifies a lossless
read-back and preserves advanced/custom settings. A default export is 23 lines.
Backgrounds are transparent by default; explicit colors in existing files remain
explicit. The old detailed schema remains supported.

1. `config` loads/normalizes TOML, expands presets, validates styles, and converts
   recognizable legacy settings without writing over source files.
2. `terminal` captures dimensions/cell pixels and bounded graphics capabilities.
3. `components::system` collects requested measurements into a typed snapshot;
   the component registry supplies labels, icon mappings, and module presets.
4. `content` resolves styles, assets, visibility, text, and aligned value columns.
5. `measurement` determines natural sizes in display cells; `layout::engine`
   allocates recursive rectangles and retains container boundaries.
6. `styles` converts configuration into Lip Gloss styles. `render` composes bounded
   Lip Gloss blocks on its canvas, then positions native images and restores the
   prompt cursor. Plain output uses the same resolved text without screen geometry.
7. `graphics` handles fitting, alpha compositing, asset reuse, and protocol encoding.

Major additions are the content/measurement modules, compatibility converter,
asset loader, component registry, terminal capability handling, CLI integration
tests, and example configurations. Existing Kitty chunking, size variants,
rectangles, configuration defaults, and the bundled image were retained/refined.

The broad generic builder abstraction was intentionally avoided: `StyleBuilder`
is used for text styles, and containers expose bounded decoration construction.
Rendering uses Lip Gloss's own canvas and alignment instead of implementing a
second ANSI cell parser or maintaining hardcoded border glyphs.

## Behavior and limits

The dashboard uses the full terminal area minus configured prompt rows, with a
minimum of one reserved row. The default is two. Tiny/custom layouts clip; only
explicitly optional nodes disappear. The default excludes CPU usage to avoid its
sampling delay. Availability failures produce `N/A` or hidden rows, not invented
measurements. Normal missing-image failures are optional; config checks report them.

Themes use palette references so changing a theme changes all default style roles.
Literal colors in older or customized configurations intentionally remain literal.
Original settings with ambiguous mappings fail explicitly and require user edits;
there is no automatic on-disk migration or config overwrite.

Native graphics implementations cover Kitty, iTerm2 inline PNG, and Sixel via
icy_sixel, with ANSI and text fallbacks. Real terminal windows were unavailable
for visual certification. Sixel colors are quantized; ANSI/Sixel transparency is
flattened onto the configured background. SVG and animation beyond a GIF's first
frame are out of scope. Tmux passthrough is opt-in and split-pane positioning is
unverified; GNU screen passthrough is unsupported.

Linux is the only runtime-tested OS. macOS/Windows rely on native portable
libraries with unavailable metrics handled gracefully; their builds and runtimes
still need external validation. Desktop, shell, terminal, GPU, battery, and display
availability varies by session and OS. The collector never invokes external commands.

## Validation

Automated tests cover defaults/serialization, nested overrides, themes/colors,
legacy conversion, metric formatting and unavailability, Unicode column alignment,
icons, border/padding geometry, allocation/overflow, raster fitting/alpha,
asset reuse, Kitty chunks, iTerm2 PNG decoding, Sixel decoding, capability parsing,
CLI parsing, redirected output, config preservation, examples, and broken pipes.

Pseudo-terminal checks exercise both orientations at 80×24, 32×12, 3×4, and 1×1;
each explicit image backend; successful simulated Kitty capability replies and
probe timeout fallback; no-color output; missing image fallback; prompt placement;
and restoration of the original termios attributes. These validate emitted bytes
and terminal state, not the visual behavior of a native emulator.

The following checks passed on Linux:

- `cargo fmt --check`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test` (55 unit tests and 9 CLI integration tests)
- `cargo build --release`
- `cargo install --offline --path . --locked --root /tmp/blink-installed-verification`
- Installed binary executed from `/tmp`, including embedded artwork and asset checks.
- `cargo package --list --allow-dirty --offline` includes the bundled image, examples,
  documentation, source, and tests. No publishing was performed.

Release measurements on this Linux machine, warm filesystem caches, ten subprocess
runs per scenario (including process startup and output consumption):

| Scenario | Median | Minimum |
| --- | ---: | ---: |
| 80×24 ANSI dashboard, explicit backend | 28.94 ms | 28.00 ms |
| Automatic dashboard, unanswered 50 ms capability probe | 90.54 ms | 79.89 ms |
| Dashboard without images | 9.31 ms | 7.71 ms |
| Plain default output | 6.97 ms | 5.99 ms |
| Explicit CPU-usage component, plain output | 210.04 ms | 206.18 ms |

Terminal scenarios used a local pseudo-terminal, not a physical emulator. These are
observations on this machine, not portable latency guarantees. The CPU-usage sample
interval and probe timeout explain their additional latency. Disable probes or choose
an explicit backend on slow links; omit CPU usage for the fastest startup.

## Future work

- Run the native graphics visual matrix in Kitty, iTerm2, and a Sixel emulator,
  including multiple images, transparency, scrolling, and tmux split panes.
- Add macOS and Windows build/runtime CI and platform-specific collector fixtures.
- Expand window-manager discovery where a reliable native API is available.
- Add SVG rasterization only with a bounded, tested dependency and explicit support.
