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


