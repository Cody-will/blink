use crate::layout::rect::Rect;

#[derive(Debug, Clone)]
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
