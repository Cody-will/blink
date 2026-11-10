pub mod node;
pub mod engine;
pub mod rect;
use node::LayoutNode;


#[derive(Debug, Clone)]
pub enum Layout {
    Horizontal {
        left: LayoutChild,
        right: LayoutChild,
    },

    Vertical {
        top: LayoutChild,
        bottom: LayoutChild,
    },
}

#[derive(Debug, Clone)]
pub struct LayoutChild {
    pub size: Size,
    pub child: LayoutNode,
}

#[derive(Debug, Clone, Copy)]
pub enum Size {
    Auto,
    Fixed(u16),
    Percent(u8),
    Fill,
}


