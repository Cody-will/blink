//! Lip Gloss builds and aligns blocks; its canvas composes them at layout coordinates.
use crate::{
    config::Backend,
    content::{Line, Resolved},
    graphics::{self, Prepared},
    layout::{engine::Placed, rect::Rect},
    styles::{Alignment, effective_background},
    terminal::CellPixels,
};
use rusty_lipgloss::{
    canvas::Canvas,
    layer::{Compositor, Layer},
    position::{place_horizontal, place_vertical},
};
use rusty_x_ansi::util::{strip, truncate};
use std::io::Write;

pub struct RenderOptions {
    pub backend: Backend,
    pub color: bool,
    pub cell: CellPixels,
    pub passthrough: bool,
}
fn clip(block: &str, width: u16, height: u16) -> String {
    block
        .lines()
        .take(usize::from(height))
        .map(|line| truncate(line, usize::from(width), ""))
        .collect::<Vec<_>>()
        .join("\n")
}
fn layer(canvas: &mut Canvas, block: &str, area: Rect, color: bool) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let clipped = clip(block, area.width, area.height);
    let text = if color { clipped } else { strip(&clipped) };
    canvas.compose(&mut Compositor::new(&[Layer::new(&text)
        .x(isize::try_from(area.x).unwrap_or(0))
        .y(isize::try_from(area.y).unwrap_or(0))]));
}
fn styled_line(line: &Line, bg: Option<&str>) -> String {
    line.iter()
        .map(|(text, s)| s.resolve_with_background(bg).render(text))
        .collect()
}
fn paint(
    canvas: &mut Canvas,
    p: &Placed<'_>,
    options: &RenderOptions,
    parent_bg: Option<&str>,
    native: &mut Vec<(Prepared, [u8; 3])>,
) {
    if p.area.width == 0 || p.area.height == 0 {
        return;
    }
    let node = p.node;
    let mut container = node.container.clone();
    container.background =
        effective_background(container.background.as_deref(), parent_bg).map(str::to_owned);
    let bg = container.background.as_deref();
    layer(
        canvas,
        &container.decoration(p.area.width, p.area.height),
        p.area,
        options.color,
    );
    let title_rows = node.title_rows().min(p.content.height);
    if let Some((title, style)) = &node.title {
        let border = node.container.border.inset();
        let area = if border > 0 {
            Rect::new(
                p.area.x.saturating_add(1),
                p.area.y,
                p.area.width.saturating_sub(2),
                1,
            )
        } else {
            Rect {
                height: title_rows,
                ..p.content
            }
        };
        let text = truncate(&format!(" {title} "), usize::from(area.width), "");
        let aligned = place_horizontal(
            usize::from(area.width),
            node.title_align.position(),
            &text,
            &[],
        );
        let prefix = aligned
            .chars()
            .take_while(|c| *c == ' ')
            .count()
            .saturating_sub(1)
            .min(usize::from(area.width)) as u16;
        let title_style = style.resolve_with_background(bg);
        layer(
            canvas,
            &title_style.render(&text),
            Rect {
                x: area.x + prefix,
                width: area.width.saturating_sub(prefix),
                ..area
            },
            options.color,
        );
    }
    if !node.lines.is_empty() {
        let area = Rect {
            y: p.content.y + title_rows,
            height: p.content.height.saturating_sub(title_rows),
            ..p.content
        };
        let lines = node
            .lines
            .iter()
            .take(usize::from(area.height))
            .map(|l| truncate(&styled_line(l, bg), usize::from(area.width), ""))
            .collect::<Vec<_>>()
            .join("\n");
        let horizontal = place_horizontal(
            usize::from(area.width),
            container.align_h.position(),
            &lines,
            &[],
        );
        let block = place_vertical(
            usize::from(area.height),
            container.align_v.position(),
            &horizontal,
            &[],
        );
        layer(canvas, &block, area, options.color);
    }
    if let Some(image) = &node.image {
        let area = Rect {
            y: p.content.y + title_rows,
            height: p.content.height.saturating_sub(title_rows),
            ..p.content
        };
        let background = bg.map(graphics::rgb).unwrap_or([0, 0, 0]);
        if options.backend == Backend::Text {
            let placeholder = "[image]";
            layer(
                canvas,
                &place_horizontal(
                    usize::from(area.width),
                    Alignment::Center.position(),
                    placeholder,
                    &[],
                ),
                Rect {
                    height: area.height.min(1),
                    ..area
                },
                false,
            );
        } else if let Some(image) =
            graphics::prepare(image, area, &node.image_options, options.cell)
        {
            if options.backend == Backend::Ansi {
                layer(
                    canvas,
                    &graphics::ansi(&image, background),
                    image.area,
                    options.color,
                );
            } else {
                native.push((image, background));
            }
        }
    }
    for child in &p.children {
        paint(canvas, child, options, bg, native);
    }
}
pub fn compose(root: &Placed<'_>, options: &RenderOptions) -> anyhow::Result<Vec<u8>> {
    if u64::from(root.area.width) * u64::from(root.area.height) > 1_000_000 {
        anyhow::bail!("Terminal area exceeds one million cells");
    }
    let mut canvas = Canvas::new(usize::from(root.area.width), usize::from(root.area.height));
    let mut native = vec![];
    paint(&mut canvas, root, options, None, &mut native);
    let rendered = canvas.render();
    let mut output = vec![];
    if root.area.height == 0 || root.area.width == 0 {
        return Ok(output);
    }
    // Clear the visible screen before placing the dashboard at the top left.
    output.write_all(b"\x1b[2J\x1b[H")?;
    // Explicit absolute rows avoid right-margin autowrap and accidental scrolling.
    for (row, line) in rendered
        .split('\n')
        .take(usize::from(root.area.height))
        .enumerate()
    {
        if row > 0 {
            write!(&mut output, "\x1b[{};1H", row + 1)?;
        }
        output.write_all(truncate(line, usize::from(root.area.width), "").as_bytes())?;
        if options.color {
            output.write_all(b"\x1b[0m")?;
        }
    }
    for (image, bg) in native {
        graphics::native(
            options.backend,
            &image,
            bg,
            options.passthrough,
            &mut output,
        )?;
    }
    write!(
        &mut output,
        "\x1b[{};1H\x1b[J",
        u32::from(root.area.height) + 1
    )?;
    Ok(output)
}
/// Plain output follows configuration order rather than terminal-height geometry.
pub fn plain(root: &Resolved) -> String {
    fn visit(n: &Resolved, out: &mut Vec<(String, bool)>) {
        if n.swatches {
            return;
        }
        if let Some((title, _)) = &n.title {
            if !out.is_empty() {
                out.push((String::new(), false));
            }
            out.push((title.clone(), false));
        }
        for line in &n.lines {
            let text = line.iter().map(|(s, _)| s.as_str()).collect::<String>();
            let art = n.kind == crate::layout::node::Kind::Ascii;
            out.push((
                if art {
                    text
                } else {
                    text.trim_end().to_owned()
                },
                art,
            ));
        }
        for child in &n.children {
            visit(child, out);
        }
    }
    let mut lines = vec![];
    visit(root, &mut lines);
    while lines
        .last()
        .is_some_and(|(line, art)| line.is_empty() && !art)
    {
        lines.pop();
    }
    format!(
        "{}\n",
        lines
            .into_iter()
            .map(|(line, _)| line)
            .collect::<Vec<_>>()
            .join("\n")
    )
}
