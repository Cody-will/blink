pub mod node;
pub mod engine;
pub mod rect;

use node::LayoutNode;
use serde::{Deserialize, Serialize};

use crate::DEFAULT_IMG_PATH;

// ---------------------------------------------
// LAYOUT
// ---------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
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
        Self::Vertical {
            top: LayoutChild::new(
                Orientation::Vertical,
                Section::Top,
            ),
            bottom: LayoutChild::new(
                Orientation::Vertical,
                Section::Bottom,
            ),
        }
    }
}

// ---------------------------------------------
// LAYOUT CHILD
// ---------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutChild {
    pub size: Size,
    pub child: LayoutNode,
}

impl LayoutChild {
    pub fn new(
        orientation: Orientation,
        section: Section,
    ) -> Self {
        let (size, child) = match (orientation, section) {
            // Vertical layout: image on top
            (Orientation::Vertical, Section::Top) => (
                Size::Percent(40),
                LayoutNode::Image {
                    path: DEFAULT_IMG_PATH.into(),
                },
            ),

            // Vertical layout: modules below
            (Orientation::Vertical, Section::Bottom) => (
                Size::Fill,
                LayoutNode::Column {
                    gap: 1,
                    children: vec![],
                },
            ),

            // Horizontal layout: image on left
            (Orientation::Horizontal, Section::Left) => (
                Size::Percent(35),
                LayoutNode::Image {
                    path: DEFAULT_IMG_PATH.into(),
                },
            ),

            // Horizontal layout: modules on right
            (Orientation::Horizontal, Section::Right) => (
                Size::Fill,
                LayoutNode::Column {
                    gap: 1,
                    children: vec![],
                },
            ),

            // Invalid combinations
            _ => unreachable!("Invalid layout orientation and section"),
        };

        Self { size, child }
    }
}

impl Default for LayoutChild {
    fn default() -> Self {
        Self {
            size: Size::default(),
            child: LayoutNode::Column {
                gap: 0,
                children: vec![],
            },
        }
    }
}

// ---------------------------------------------
// SIZE
// ---------------------------------------------

#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Size {
    Auto,
    Fixed(u16),
    Percent(u8),

    #[default]
    Fill,
}

// ---------------------------------------------
// ORIENTATION / SECTION
// ---------------------------------------------

#[derive(Debug, Clone, Copy)]
pub enum Orientation {
    Vertical,
    Horizontal,
}

#[derive(Debug, Clone, Copy)]
pub enum Section {
    Top,
    Bottom,
    Left,
    Right,
}

