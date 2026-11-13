pub mod theme;

use serde::{Deserialize, Serialize};

use crate::layout::node::LayoutNode;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "direction", rename_all = "snake_case")]
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

impl Default for Layout {
    fn default() -> Self {
        Self::Horizontal {
            left: LayoutChild {
                size: Size::Percent(35),
                child: LayoutNode::Image {
                    path: String::new(),
                },
            },
            right: LayoutChild {
                size: Size::Fill,
                child: LayoutNode::Column {
                    gap: 0,
                    children: Vec::new(),
                },
            },
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LayoutChild {
    pub size: Size,
    pub child: LayoutNode,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Size {
    Auto,
    Fixed(u16),
    Percent(u8),
    Fill,
}
