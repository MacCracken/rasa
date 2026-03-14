use serde::{Deserialize, Serialize};

use crate::geometry::{Point, Rect};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub enum Selection {
    #[default]
    None,
    Rect(Rect),
    Ellipse(Rect),
    Freeform {
        points: Vec<Point>,
    },
    Mask {
        width: u32,
        height: u32,
    },
}
