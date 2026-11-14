pub mod engine;
pub mod node;
pub mod rect;

use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize, PartialEq, clap::ValueEnum)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    Horizontal,
    #[default]
    Vertical,
}

#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Size {
    Auto,
    Fixed(u16),
    Percent(u8),
    #[default]
    Fill,
}
