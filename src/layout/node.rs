use crate::{
    layout::{Direction, Size},
    styles::Alignment,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Node {
    #[serde(rename = "type")]
    pub kind: Kind,
    pub size: Size,
    pub direction: Direction,
    pub gap: u16,
    /// Child placement, separate from styling/alignment of text inside children.
    pub align_h: Option<Alignment>,
    pub align_v: Option<Alignment>,
    pub title: String,
    pub title_align: Alignment,
    pub content: String,
    pub name: String,
    pub preset: Option<String>,
    pub label: Option<String>,
    pub value: Option<String>,
    pub icon: Option<String>,
    pub icon_path: Option<String>,
    pub path: Option<String>,
    pub visible: bool,
    pub optional: bool,
    pub image: ImageOptions,
    pub children: Vec<Node>,
    pub style: toml::Table,
    pub text_style: toml::Table,
    pub label_style: toml::Table,
    pub icon_style: toml::Table,
}
impl Default for Node {
    fn default() -> Self {
        Self {
            kind: Kind::Column,
            size: Size::Fill,
            direction: Direction::Vertical,
            gap: 0,
            align_h: None,
            align_v: None,
            title: String::new(),
            title_align: Alignment::Center,
            content: String::new(),
            name: String::new(),
            preset: None,
            label: None,
            value: None,
            icon: None,
            icon_path: None,
            path: None,
            visible: true,
            optional: false,
            image: ImageOptions::default(),
            children: vec![],
            style: toml::Table::new(),
            text_style: toml::Table::new(),
            label_style: toml::Table::new(),
            icon_style: toml::Table::new(),
        }
    }
}
#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Row,
    #[default]
    Column,
    Module,
    Image,
    Text,
    Component,
    Swatches,
}
#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Fit {
    #[default]
    Contain,
    Cover,
    Stretch,
}
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct ImageOptions {
    pub fit: Fit,
    pub align_h: Alignment,
    pub align_v: Alignment,
}
impl Default for ImageOptions {
    fn default() -> Self {
        Self {
            fit: Fit::Contain,
            align_h: Alignment::Center,
            align_v: Alignment::Center,
        }
    }
}
