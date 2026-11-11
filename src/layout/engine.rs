use crate::layout::{Layout, node::{LayoutNode, RenderNode}, rect::Rect};


pub struct LayoutEngine;

impl LayoutEngine {
    pub fn calculate<'a>(layout: &'a Layout, area: Rect) -> Vec<RenderNode<'a>> {
        let mut items = Vec::new();

        match layout {
            Layout::Horizontal { left, right } => {
                let (left_area, right_area) = area.horizontal_split();
                Self::place_node(&left.child, left_area, &mut items);
                Self::place_node(&right.child, right_area, &mut items);
            },
            Layout::Vertical { top, bottom } => {
                let (top_area, bottom_area) = area.vertical_split();
                Self::place_node(&top.child, top_area, &mut items);
                Self::place_node(&bottom.child, bottom_area, &mut items);
            },
        }
        items
    }
    
    fn place_node<'a>(node: &'a LayoutNode, area: Rect, placed: &mut Vec<RenderNode<'a>>) {
        match node {
            LayoutNode::Text { .. } | LayoutNode::Module { .. } => {
                placed.push(RenderNode { node, area });
            },
            LayoutNode::Column { .. } => {},
            LayoutNode::Row { .. } => {},
            LayoutNode::Icon { .. } => {},
            LayoutNode::Image { .. } => {},
        }
    }
}
