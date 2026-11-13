use crate::styles::ContainerStyleConfig;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

#[allow(dead_code)] 
impl Rect {
    pub fn new(x: u16, y: u16, width: u16, height: u16) -> Self {
        Self { x, y, width, height }
    }

    pub fn horizontal_split(&self) -> (Rect, Rect) {
       let left_width = self.width / 2;

       let left = Rect{
           x: self.x,
           y: self.y, 
           width: left_width, 
           height: self.height
       };

       let right = Rect{
           x: self.x + left_width,
           y: self.y, 
           width: self.width - left_width, 
           height: self.height
       };

       (left, right)
    }

    pub fn vertical_split(&self) -> (Rect, Rect) {
        let top_height = self.height / 2;

        let top = Rect {
            x: self.x,
            y: self.y,
            width: self.width,
            height: top_height,
        };

        let bottom = Rect {
            x: self.x,
            y: self.y + top_height,
            width: self.width,
            height: self.height - top_height,
        };

        (top, bottom)
    }

    pub fn inner(&self, container: &ContainerStyleConfig) -> Self {
        let border = container.border.inset();
        let padding = container.padding;

        let left = border.saturating_add(padding.left);
        let right = border.saturating_add(padding.right);
        let top = border.saturating_add(padding.top);
        let bottom = border.saturating_add(padding.bottom);

        Self {
            x: self.x.saturating_add(left),
            y: self.y.saturating_add(top),
            width: self.width.saturating_add(left.saturating_add(right)),
            height: self.height.saturating_add(top.saturating_add(bottom)),
        }
    }
}



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn horizontal_split_preserves_width() {
        let rect = Rect::new(10, 5, 121, 30);
        let (left, right) = rect.horizontal_split();

        assert_eq!(left, Rect::new(10, 5, 60, 30));
        assert_eq!(right, Rect::new(70, 5, 61, 30));
        assert_eq!(left.width + right.width, rect.width);
    }

    #[test]
    fn vertical_split_preserves_height() {
        let rect = Rect::new(10, 5, 120, 25);
        let (top, bottom) = rect.vertical_split();

        assert_eq!(top, Rect::new(10, 5, 120, 12));
        assert_eq!(bottom, Rect::new(10, 17, 120, 13));
        assert_eq!(top.height + bottom.height, rect.height);
    }
}
