//! Content measurement in terminal cells, independent of ANSI byte lengths.
use crate::{content::Resolved, layout::Direction};
use unicode_width::UnicodeWidthStr;
impl Resolved {
    pub fn measure(&self) -> (u16, u16) {
        let mut width = self
            .lines
            .iter()
            .map(|l| l.iter().map(|(s, _)| s.width()).sum::<usize>())
            .max()
            .unwrap_or(0)
            .min(u16::MAX as usize) as u16;
        let mut height =
            (self.lines.len().min(u16::MAX as usize) as u16).saturating_add(self.title_rows());
        let (mut cw, mut ch) = (0u16, 0u16);
        for c in &self.children {
            let (mut w, mut h) = c.measure();
            if let crate::layout::Size::Fixed(n) = c.size {
                if self.direction == Direction::Horizontal {
                    w = n;
                } else {
                    h = n;
                }
            }
            if self.direction == Direction::Horizontal {
                cw = cw.saturating_add(w);
                ch = ch.max(h);
            } else {
                cw = cw.max(w);
                ch = ch.saturating_add(h);
            }
        }
        let gaps = self
            .gap
            .saturating_mul(self.children.len().saturating_sub(1).min(u16::MAX as usize) as u16);
        if self.direction == Direction::Horizontal {
            cw = cw.saturating_add(gaps);
        } else {
            ch = ch.saturating_add(gaps);
        }
        if let Some(image) = &self.image {
            cw = cw.max(self.image_columns);
            ch = ch.max(
                ((f64::from(self.image_columns) * f64::from(image.height())
                    / f64::from(image.width())
                    / self.cell_ratio)
                    .ceil() as u16)
                    .max(1),
            );
        }
        width = width.max(cw);
        height = height.saturating_add(ch);
        if let Some((title, _)) = &self.title {
            width = width.max((title.width() + 2).min(u16::MAX as usize) as u16);
        }
        (
            width.saturating_add(self.container.horizontal_inset()),
            height.saturating_add(self.container.vertical_inset()),
        )
    }
}
