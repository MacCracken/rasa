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
    /// Grayscale mask where each pixel is 0.0 (unselected) to 1.0 (selected).
    Mask {
        width: u32,
        height: u32,
        data: Vec<f32>,
    },
}

/// How to combine a new selection with an existing one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SelectionOp {
    Replace,
    Add,
    Subtract,
    Intersect,
}

impl Selection {
    pub fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }

    /// Check if a point is inside the selection.
    pub fn contains(&self, point: Point) -> bool {
        match self {
            Self::None => true, // no selection = everything selected
            Self::Rect(r) => r.contains(point),
            Self::Ellipse(r) => {
                let cx = r.x + r.width / 2.0;
                let cy = r.y + r.height / 2.0;
                let rx = r.width / 2.0;
                let ry = r.height / 2.0;
                if rx <= 0.0 || ry <= 0.0 {
                    return false;
                }
                let dx = (point.x - cx) / rx;
                let dy = (point.y - cy) / ry;
                dx * dx + dy * dy <= 1.0
            }
            Self::Freeform { points } => point_in_polygon(point, points),
            Self::Mask {
                width,
                height,
                data,
            } => {
                let x = point.x as u32;
                let y = point.y as u32;
                if x < *width && y < *height {
                    let idx = (y as usize) * (*width as usize) + (x as usize);
                    data[idx] > 0.5
                } else {
                    false
                }
            }
        }
    }

    /// Get the bounding rect of the selection.
    pub fn bounds(&self) -> Option<Rect> {
        match self {
            Self::None => None,
            Self::Rect(r) | Self::Ellipse(r) => Some(*r),
            Self::Freeform { points } => {
                if points.is_empty() {
                    return None;
                }
                let mut min_x = f64::MAX;
                let mut min_y = f64::MAX;
                let mut max_x = f64::MIN;
                let mut max_y = f64::MIN;
                for p in points {
                    min_x = min_x.min(p.x);
                    min_y = min_y.min(p.y);
                    max_x = max_x.max(p.x);
                    max_y = max_y.max(p.y);
                }
                Some(Rect {
                    x: min_x,
                    y: min_y,
                    width: max_x - min_x,
                    height: max_y - min_y,
                })
            }
            Self::Mask {
                width,
                height,
                data,
            } => {
                let mut min_x = *width;
                let mut min_y = *height;
                let mut max_x = 0u32;
                let mut max_y = 0u32;
                for y in 0..*height {
                    for x in 0..*width {
                        let idx = (y as usize) * (*width as usize) + (x as usize);
                        if data[idx] > 0.5 {
                            min_x = min_x.min(x);
                            min_y = min_y.min(y);
                            max_x = max_x.max(x);
                            max_y = max_y.max(y);
                        }
                    }
                }
                if max_x < min_x {
                    None
                } else {
                    Some(Rect {
                        x: min_x as f64,
                        y: min_y as f64,
                        width: (max_x - min_x + 1) as f64,
                        height: (max_y - min_y + 1) as f64,
                    })
                }
            }
        }
    }

    /// Invert the selection (only works for mask-based selections).
    pub fn invert(&self, width: u32, height: u32) -> Self {
        match self {
            Self::None => {
                // Invert "everything" = "nothing" — but we represent as empty mask
                Self::Mask {
                    width,
                    height,
                    data: vec![0.0; (width as usize) * (height as usize)],
                }
            }
            Self::Mask {
                width: w,
                height: h,
                data,
            } => Self::Mask {
                width: *w,
                height: *h,
                data: data.iter().map(|v| 1.0 - v).collect(),
            },
            other => {
                // Convert to mask first, then invert
                let mask = other.to_mask(width, height);
                mask.invert(width, height)
            }
        }
    }

    /// Convert any selection to a mask representation.
    pub fn to_mask(&self, width: u32, height: u32) -> Self {
        match self {
            Self::None => Self::Mask {
                width,
                height,
                data: vec![1.0; (width as usize) * (height as usize)],
            },
            Self::Mask { .. } => self.clone(),
            _ => {
                let mut data = vec![0.0_f32; (width as usize) * (height as usize)];
                for y in 0..height {
                    for x in 0..width {
                        if self.contains(Point {
                            x: x as f64 + 0.5,
                            y: y as f64 + 0.5,
                        }) {
                            data[(y as usize) * (width as usize) + (x as usize)] = 1.0;
                        }
                    }
                }
                Self::Mask {
                    width,
                    height,
                    data,
                }
            }
        }
    }
}

/// Ray-casting point-in-polygon test.
fn point_in_polygon(point: Point, polygon: &[Point]) -> bool {
    if polygon.len() < 3 {
        return false;
    }
    let mut inside = false;
    let n = polygon.len();
    let mut j = n - 1;
    for i in 0..n {
        let pi = &polygon[i];
        let pj = &polygon[j];
        if ((pi.y > point.y) != (pj.y > point.y))
            && (point.x < (pj.x - pi.x) * (point.y - pi.y) / (pj.y - pi.y) + pi.x)
        {
            inside = !inside;
        }
        j = i;
    }
    inside
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn none_contains_everything() {
        let sel = Selection::None;
        assert!(sel.contains(Point { x: 100.0, y: 100.0 }));
    }

    #[test]
    fn rect_contains() {
        let sel = Selection::Rect(Rect {
            x: 10.0,
            y: 10.0,
            width: 20.0,
            height: 20.0,
        });
        assert!(sel.contains(Point { x: 15.0, y: 15.0 }));
        assert!(!sel.contains(Point { x: 5.0, y: 5.0 }));
    }

    #[test]
    fn ellipse_contains() {
        let sel = Selection::Ellipse(Rect {
            x: 0.0,
            y: 0.0,
            width: 20.0,
            height: 20.0,
        });
        // Center
        assert!(sel.contains(Point { x: 10.0, y: 10.0 }));
        // Corner should be outside
        assert!(!sel.contains(Point { x: 0.5, y: 0.5 }));
    }

    #[test]
    fn freeform_triangle() {
        let sel = Selection::Freeform {
            points: vec![
                Point { x: 0.0, y: 0.0 },
                Point { x: 10.0, y: 0.0 },
                Point { x: 5.0, y: 10.0 },
            ],
        };
        assert!(sel.contains(Point { x: 5.0, y: 3.0 }));
        assert!(!sel.contains(Point { x: 0.0, y: 10.0 }));
    }

    #[test]
    fn mask_contains() {
        let sel = Selection::Mask {
            width: 4,
            height: 4,
            data: vec![
                1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
            ],
        };
        assert!(sel.contains(Point { x: 0.0, y: 0.0 }));
        assert!(sel.contains(Point { x: 1.0, y: 1.0 }));
        assert!(!sel.contains(Point { x: 1.0, y: 0.0 }));
    }

    #[test]
    fn bounds_rect() {
        let sel = Selection::Rect(Rect {
            x: 5.0,
            y: 10.0,
            width: 20.0,
            height: 30.0,
        });
        let b = sel.bounds().unwrap();
        assert_eq!(b.x, 5.0);
        assert_eq!(b.width, 20.0);
    }

    #[test]
    fn bounds_none() {
        assert!(Selection::None.bounds().is_none());
    }

    #[test]
    fn to_mask_rect() {
        let sel = Selection::Rect(Rect {
            x: 1.0,
            y: 1.0,
            width: 2.0,
            height: 2.0,
        });
        let mask = sel.to_mask(4, 4);
        if let Selection::Mask { data, .. } = &mask {
            // Pixel (1,1) should be selected (center at 1.5, 1.5 is inside)
            assert_eq!(data[5], 1.0);
            // Pixel (0,0) should not be selected (center at 0.5, 0.5 is outside)
            assert_eq!(data[0], 0.0);
        } else {
            panic!("expected mask");
        }
    }

    #[test]
    fn invert_mask() {
        let sel = Selection::Mask {
            width: 2,
            height: 2,
            data: vec![1.0, 0.0, 0.0, 1.0],
        };
        let inverted = sel.invert(2, 2);
        if let Selection::Mask { data, .. } = &inverted {
            assert_eq!(data, &[0.0, 1.0, 1.0, 0.0]);
        } else {
            panic!("expected mask");
        }
    }

    #[test]
    fn is_none() {
        assert!(Selection::None.is_none());
        assert!(
            !Selection::Rect(Rect {
                x: 0.0,
                y: 0.0,
                width: 1.0,
                height: 1.0
            })
            .is_none()
        );
    }
}
