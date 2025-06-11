use crate::domain::color::Color;
use crate::domain::position::Position;

#[derive(Clone, Debug)]
pub struct TextOverlay {
    pub text: String,
    pub color: Color,
    pub position: Position,
}

impl TextOverlay {
    pub fn new(text: String, color: Color, position: Position) -> Self {
        Self {
            text,
            color,
            position,
        }
    }
}
