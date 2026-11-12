use crate::layout::{Layout, node::{LayoutNode, RenderNode}, rect::Rect};


pub struct LayoutEngine;

impl LayoutEngine {
    pub fn calculate<'a>(layout: &'a Layout, area: Rect) -> Vec<RenderNode<'a>> {
        let mut items = Vec::new();

        match layout {
            Layout::Horizontal { left, right } => {
                let (left_area, right_area) = area.horizontal_split();
                Self::place_node(&left.child, left_area, &mut items);
                Self::place_node(&right.child, right_area, &mut items);
            },
            Layout::Vertical { top, bottom } => {
                let (top_area, bottom_area) = area.vertical_split();
                Self::place_node(&top.child, top_area, &mut items);
                Self::place_node(&bottom.child, bottom_area, &mut items);
            },
        }
        items
    }
    
    fn place_node<'a>(node: &'a LayoutNode, area: Rect, placed: &mut Vec<RenderNode<'a>>) {
        match node {
            LayoutNode::Text { .. }
            | LayoutNode::Module { .. } 
            | LayoutNode::Icon { .. }
            | LayoutNode::Image { .. } => {
                placed.push(RenderNode { node, area });
            },  
            LayoutNode::Column { gap, children } => {
                if children.is_empty() {
                    return;
                }

                let count = children.len() as u32;
                let total_gap = (*gap as u32).saturating_mul(count - 1);
                let usable_height = (area.height as u32).saturating_sub(total_gap);
                
                let child_height = usable_height / count;
                let remainder = usable_height % count;

                let mut y = area.y as u32;
                for (index, child) in children.iter().enumerate() {
                    let height = child_height + u32::from((index as u32) < remainder);
                    let child_area = Rect {
                        x: area.x,
                        y: y as u16,
                        width: area.width,
                        height: height as u16,
                    };

                    Self::place_node(child, child_area, placed);

                    y += height + *gap as u32;
                }
            },
            LayoutNode::Row { gap, children } => {
               if children.is_empty() {
                   return;
               }

               let count = children.len() as u32;
               let total_gap = (*gap as u32).saturating_mul(count - 1);
               let usable_width = (area.width as u32).saturating_sub(total_gap);

               let child_width = usable_width / count;
               let remainder = usable_width % count; 

               let mut x = area.x as u32;  

               for (index, child) in children.iter().enumerate() {
                   let width = child_width + u32::from((index as u32) < remainder); 
                   let child_area = Rect {
                       x: x as u16, 
                       y: area.y, 
                       width: width as u16,
                       height: area.height,
                   };

                   Self::place_node(child, child_area, placed);

                   x += width + *gap as u32;
               }
            }, 
        }
    }
}
