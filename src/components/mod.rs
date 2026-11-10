pub mod system;


#[derive(Debug, Clone, Copy)]
pub struct Size {
    pub width: u16,
    pub height: u16,
}


#[derive(Debug, Clone)]
pub struct TextComponent {
    pub label: String,
    pub content: String,
}


impl TextComponent {
    fn from(label: String, content: String) -> Self {
        Self { label, content }
    }
}


impl Component for TextComponent {
    fn content(&self) -> &str {
        &self.content
    } 
    
}


#[derive(Debug, Clone)]
pub struct HeaderComponent {
    pub content: String,
}


impl Component for HeaderComponent {
    fn content(&self) -> &str {
        &self.content
    }
}


pub trait Component {
    fn content(&self) -> &str;
    
    fn render(&self) -> String {
        self.content().to_string()
    }

    fn measure(&self) -> Size {
        let output = self.render();

        Size {
            width: output 
                .lines() 
                .map(|line| line.chars().count())
                .max()
                .unwrap_or(0) as u16,
            height: output.lines().count() as u16,
        }
    }
}
