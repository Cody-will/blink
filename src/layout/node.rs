use crate::layout::rect::Rect;
use serde::{Deserialize, Serialize};


#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum LayoutNode {
    Row {
        gap: u16,
        children: Vec<LayoutNode>,
    },
    Column {
        gap: u16,
        children: Vec<LayoutNode>,
    },
    Text {
        content: String,
    },
    Module {
        name: String,
    },
    Image {
        path: String,
    },
    Icon {
        path: String,
    },
}


#[derive(Debug, Clone, Copy)]
pub struct RenderNode<'a> {
    pub node: &'a LayoutNode,
    pub area: Rect,
}
