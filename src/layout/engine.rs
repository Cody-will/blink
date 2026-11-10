use crate::layout::{Layout, rect::Rect};


pub struct LayoutEngine;

impl LayoutEngine {
    pub fn calculate(layout: &Layout, area: Rect) {
        match layout {
            Layout::Horizontal { left, right } => {
                let (left_area, right_area) = area.horizontal_split();
            },
            Layout::Vertical { top, bottom } => {
                let (top_area, bottom_area) = area.vertical_split();
            }
        }
    }
}
