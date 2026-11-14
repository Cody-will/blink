mod components;
mod config;
mod content;
mod graphics;
mod layout;
mod measurement;
mod render;
mod styles;
mod terminal;

use anyhow::Context;
use clap::Parser;
use config::{Backend, Config};
use layout::{Direction, rect::Rect};
use std::{
    io::{self, IsTerminal, Write},
    path::PathBuf,
};

#[derive(Debug, Parser)]
#[command(version, about = "A configurable, print-once system dashboard")]
struct Args {
    /// TOML configuration path.
    #[arg(short, long)]
    config: Option<PathBuf>,
    /// Override the configured theme without modifying the file.
    #[arg(long)]
    theme: Option<String>,
    /// Override the primary split direction.
    #[arg(long, value_enum)]
    layout: Option<Direction>,
    /// Override the selected graphics backend (TTY output only).
    #[arg(long, value_enum)]
    graphics: Option<Backend>,
    #[arg(long)]
    no_image: bool,
    #[arg(long)]
    no_icons: bool,
    /// Disable colors and native/ANSI images; retain dashboard geometry.
    #[arg(long)]
    no_color: bool,
    /// Emit readable text without borders, graphics, or terminal controls.
    #[arg(long)]
    plain: bool,
    /// Print a concise, editable configuration without collecting system information.
    #[arg(long,conflicts_with_all=["list_themes","list_modules","check_config"])]
    print_config: bool,
    #[arg(long,conflicts_with_all=["list_modules","check_config"])]
    list_themes: bool,
    #[arg(long, conflicts_with = "check_config")]
    list_modules: bool,
    /// Validate configuration and image assets without rendering.
    #[arg(long)]
    check_config: bool,
    /// Override terminal width in cells.
    #[arg(long)]
    width: Option<u16>,
    /// Override terminal height, including reserved prompt rows.
    #[arg(long)]
    height: Option<u16>,
}
fn run() -> anyhow::Result<()> {
    let args = Args::parse();
    let stdout = io::stdout();
    let is_terminal = stdout.is_terminal();
    let mut output = stdout.lock();
    if args.list_themes {
        writeln!(output, "{}", config::theme::NAMES.join("\n"))?;
        return Ok(());
    }
    if args.list_modules {
        for name in ["system", "hardware", "storage", "session"] {
            if let Some((title, keys)) = components::preset(name) {
                writeln!(output, "{name} ({title}): {}", keys.join(", "))?;
            }
        }
        writeln!(output, "\nComponents:")?;
        for d in components::DEFINITIONS {
            writeln!(output, "{}\t{}", d.name, d.label)?;
        }
        return Ok(());
    }
    let inspect = args.print_config || args.check_config;
    let (mut cfg, path) = Config::load(args.config.as_deref(), !inspect)?;
    if let Some(theme) = args.theme {
        cfg.theme = theme;
    }
    if let Some(direction) = args.layout {
        cfg.layout.direction = direction;
    }
    if let Some(backend) = args.graphics {
        cfg.graphics = backend;
    }
    if args.no_icons {
        cfg.icons.enabled = false;
    }
    cfg.validate()?;
    if args.print_config {
        content::prepare(
            &mut cfg,
            u16::MAX,
            u16::MAX,
            args.no_image,
            args.no_icons,
            false,
        );
        output.write_all(cfg.editable_toml()?.as_bytes())?;
        return Ok(());
    }
    let base = path.parent().unwrap_or_else(|| std::path::Path::new("."));
    let mut assets = graphics::assets::Assets::default();
    if args.check_config {
        fn check(
            n: &layout::node::Node,
            assets: &mut graphics::assets::Assets,
            base: &std::path::Path,
        ) {
            if !n.visible {
                return;
            }
            if n.kind == layout::node::Kind::Image {
                assets.load(n.path.as_deref(), base);
            }
            if let Some(path) = &n.icon_path {
                assets.load(Some(path), base);
            }
            for c in &n.children {
                check(c, assets, base);
            }
        }
        check(&cfg.layout.first, &mut assets, base);
        check(&cfg.layout.second, &mut assets, base);
        if !assets.warnings.is_empty() {
            anyhow::bail!(
                "{}",
                assets.warnings.into_iter().collect::<Vec<_>>().join("\n")
            );
        }
        writeln!(output, "Configuration is valid")?;
        return Ok(());
    }
    let plain = args.plain || !is_terminal || std::env::var("TERM").is_ok_and(|t| t == "dumb");
    let color = !args.no_color && std::env::var_os("NO_COLOR").is_none() && !plain;
    let term = if plain {
        terminal::Terminal {
            width: 80,
            height: 24,
            cell: terminal::CellPixels {
                width: cfg.display.cell_width,
                height: cfg.display.cell_height,
            },
        }
    } else {
        terminal::Terminal::detect(&cfg.display)
    };
    let width = args.width.unwrap_or(term.width);
    // Always leave at least one row for the returning prompt, even if reserved_rows is zero.
    let height = args
        .height
        .unwrap_or(term.height)
        .saturating_sub(cfg.reserved_rows.max(1));
    if !plain && u64::from(width) * u64::from(height) > 1_000_000 {
        anyhow::bail!("Terminal area exceeds one million cells");
    }
    cfg.display.cell_width = term.cell.width;
    cfg.display.cell_height = term.cell.height;
    content::prepare(&mut cfg, width, height, args.no_image, args.no_icons, plain);
    let keys = components::system::Snapshot::requested([&cfg.layout.first, &cfg.layout.second]);
    let snapshot = components::system::Snapshot::collect(&keys);
    let root = content::resolve(&cfg, &snapshot, base, &mut assets)?;
    if plain {
        output.write_all(render::plain(&root).as_bytes())?;
    } else {
        fn images(n: &content::Resolved) -> bool {
            n.image.is_some() || n.children.iter().any(images)
        }
        let backend = terminal::backend(
            cfg.graphics,
            is_terminal,
            images(&root),
            color,
            &cfg.display,
        )?;
        let options = render::RenderOptions {
            backend,
            color,
            cell: term.cell,
            passthrough: cfg.display.multiplexer_passthrough && std::env::var_os("TMUX").is_some(),
        };
        let bytes = render::compose(&root.place(Rect::new(0, 0, width, height)), &options)?;
        output
            .write_all(&bytes)
            .context("Writing terminal output")?;
    }
    output.flush()?;
    for warning in &assets.warnings {
        eprintln!("blink: {warning}");
    }
    Ok(())
}
fn main() -> std::process::ExitCode {
    match run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error)
            if error
                .chain()
                .filter_map(|e| e.downcast_ref::<io::Error>())
                .any(|e| e.kind() == io::ErrorKind::BrokenPipe) =>
        {
            std::process::ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("blink: {error:#}");
            std::process::ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests;
