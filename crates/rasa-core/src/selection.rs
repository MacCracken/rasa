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
                if point.x < 0.0 || point.y < 0.0 {
                    return false;
                }
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

    /// Combine this selection with another using the given operation.
    /// Both selections are converted to masks for pixel-accurate combining.
    pub fn combine(&self, other: &Selection, op: SelectionOp, width: u32, height: u32) -> Self {
        match op {
            SelectionOp::Replace => other.clone(),
            SelectionOp::Add | SelectionOp::Subtract | SelectionOp::Intersect => {
                let a = self.to_mask(width, height);
                let b = other.to_mask(width, height);
                if let (Self::Mask { data: da, .. }, Self::Mask { data: db, .. }) = (&a, &b) {
                    let data = da
                        .iter()
                        .zip(db.iter())
                        .map(|(&va, &vb)| match op {
                            SelectionOp::Add => (va + vb).min(1.0),
                            SelectionOp::Subtract => (va - vb).max(0.0),
                            SelectionOp::Intersect => va.min(vb),
                            SelectionOp::Replace => unreachable!(),
                        })
                        .collect();
                    Self::Mask {
                        width,
                        height,
                        data,
                    }
                } else {
                    // to_mask always returns Mask variant
                    unreachable!()
                }
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
            Self::Rect(r) => {
                let mut data = vec![0.0_f32; (width as usize) * (height as usize)];
                let x0 = (r.x.max(0.0) as u32).min(width);
                let y0 = (r.y.max(0.0) as u32).min(height);
                let x1 = ((r.x + r.width).ceil() as u32).min(width);
                let y1 = ((r.y + r.height).ceil() as u32).min(height);
                for y in y0..y1 {
                    for x in x0..x1 {
                        data[(y as usize) * (width as usize) + (x as usize)] = 1.0;
                    }
                }
                Self::Mask {
                    width,
                    height,
                    data,
                }
            }
            Self::Ellipse(r) => {
                let mut data = vec![0.0_f32; (width as usize) * (height as usize)];
                let cx = r.x + r.width / 2.0;
                let cy = r.y + r.height / 2.0;
                let rx = r.width / 2.0;
                let ry = r.height / 2.0;
                if rx > 0.0 && ry > 0.0 {
                    // Only iterate over the bounding box
                    let x0 = (r.x.max(0.0) as u32).min(width);
                    let y0 = (r.y.max(0.0) as u32).min(height);
                    let x1 = ((r.x + r.width).ceil() as u32).min(width);
                    let y1 = ((r.y + r.height).ceil() as u32).min(height);
                    for y in y0..y1 {
                        for x in x0..x1 {
                            let dx = (x as f64 + 0.5 - cx) / rx;
                            let dy = (y as f64 + 0.5 - cy) / ry;
                            if dx * dx + dy * dy <= 1.0 {
                                data[(y as usize) * (width as usize) + (x as usize)] = 1.0;
                            }
                        }
                    }
                }
                Self::Mask {
                    width,
                    height,
                    data,
                }
            }
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
    fn mask_rejects_negative_coords() {
        let sel = Selection::Mask {
            width: 4,
            height: 4,
            data: vec![1.0; 16],
        };
        assert!(!sel.contains(Point { x: -1.0, y: 0.0 }));
        assert!(!sel.contains(Point { x: 0.0, y: -1.0 }));
        assert!(!sel.contains(Point { x: -5.0, y: -5.0 }));
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

    // ── Combine operations ──

    #[test]
    fn combine_replace() {
        let a = Selection::Rect(Rect {
            x: 0.0,
            y: 0.0,
            width: 5.0,
            height: 5.0,
        });
        let b = Selection::Rect(Rect {
            x: 3.0,
            y: 3.0,
            width: 5.0,
            height: 5.0,
        });
        let result = a.combine(&b, SelectionOp::Replace, 10, 10);
        // Replace should just return b
        assert!(result.contains(Point { x: 5.0, y: 5.0 }));
        assert!(!result.contains(Point { x: 1.0, y: 1.0 }));
    }

    #[test]
    fn combine_add() {
        let a = Selection::Rect(Rect {
            x: 0.0,
            y: 0.0,
            width: 5.0,
            height: 5.0,
        });
        let b = Selection::Rect(Rect {
            x: 3.0,
            y: 3.0,
            width: 5.0,
            height: 5.0,
        });
        let result = a.combine(&b, SelectionOp::Add, 10, 10);
        // Both regions should be selected
        assert!(result.contains(Point { x: 1.0, y: 1.0 }));
        assert!(result.contains(Point { x: 6.0, y: 6.0 }));
        // Outside both should not
        assert!(!result.contains(Point { x: 9.0, y: 0.0 }));
    }

    #[test]
    fn combine_subtract() {
        let a = Selection::Rect(Rect {
            x: 0.0,
            y: 0.0,
            width: 8.0,
            height: 8.0,
        });
        let b = Selection::Rect(Rect {
            x: 4.0,
            y: 0.0,
            width: 8.0,
            height: 8.0,
        });
        let result = a.combine(&b, SelectionOp::Subtract, 10, 10);
        // Left part of A should remain
        assert!(result.contains(Point { x: 1.0, y: 1.0 }));
        // Overlapping part should be removed
        assert!(!result.contains(Point { x: 5.0, y: 1.0 }));
    }

    #[test]
    fn combine_intersect() {
        let a = Selection::Rect(Rect {
            x: 0.0,
            y: 0.0,
            width: 6.0,
            height: 6.0,
        });
        let b = Selection::Rect(Rect {
            x: 3.0,
            y: 3.0,
            width: 6.0,
            height: 6.0,
        });
        let result = a.combine(&b, SelectionOp::Intersect, 10, 10);
        // Only the overlap should be selected
        assert!(result.contains(Point { x: 4.0, y: 4.0 }));
        // A-only region should not be selected
        assert!(!result.contains(Point { x: 1.0, y: 1.0 }));
        // B-only region should not be selected
        assert!(!result.contains(Point { x: 7.0, y: 7.0 }));
    }

    #[test]
    fn combine_add_with_none() {
        let a = Selection::None;
        let b = Selection::Rect(Rect {
            x: 2.0,
            y: 2.0,
            width: 4.0,
            height: 4.0,
        });
        // None = everything selected, so add should be everything
        let result = a.combine(&b, SelectionOp::Add, 8, 8);
        assert!(result.contains(Point { x: 0.0, y: 0.0 }));
        assert!(result.contains(Point { x: 4.0, y: 4.0 }));
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

    #[test]
    fn ellipse_zero_radius_contains_nothing() {
        let sel = Selection::Ellipse(Rect {
            x: 5.0,
            y: 5.0,
            width: 0.0,
            height: 0.0,
        });
        assert!(!sel.contains(Point { x: 5.0, y: 5.0 }));
    }

    #[test]
    fn bounds_freeform() {
        let sel = Selection::Freeform {
            points: vec![
                Point { x: 1.0, y: 2.0 },
                Point { x: 5.0, y: 2.0 },
                Point { x: 3.0, y: 8.0 },
            ],
        };
        let b = sel.bounds().unwrap();
        assert_eq!(b.x, 1.0);
        assert_eq!(b.y, 2.0);
        assert_eq!(b.width, 4.0);
        assert_eq!(b.height, 6.0);
    }

    #[test]
    fn bounds_freeform_empty() {
        let sel = Selection::Freeform { points: vec![] };
        assert!(sel.bounds().is_none());
    }

    #[test]
    fn bounds_mask() {
        let sel = Selection::Mask {
            width: 4,
            height: 4,
            data: vec![
                0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0,
            ],
        };
        let b = sel.bounds().unwrap();
        assert_eq!(b.x, 1.0);
        assert_eq!(b.y, 1.0);
        assert_eq!(b.width, 2.0);
        assert_eq!(b.height, 2.0);
    }

    #[test]
    fn bounds_empty_mask() {
        let sel = Selection::Mask {
            width: 2,
            height: 2,
            data: vec![0.0; 4],
        };
        assert!(sel.bounds().is_none());
    }

    #[test]
    fn invert_none() {
        let sel = Selection::None;
        let inverted = sel.invert(4, 4);
        // Inverting "everything" should give empty mask
        if let Selection::Mask { data, .. } = &inverted {
            assert!(data.iter().all(|&v| v == 0.0));
        } else {
            panic!("expected Mask");
        }
    }

    #[test]
    fn invert_rect_via_mask() {
        let sel = Selection::Rect(Rect {
            x: 1.0,
            y: 1.0,
            width: 2.0,
            height: 2.0,
        });
        let inverted = sel.invert(4, 4);
        if let Selection::Mask { data, .. } = &inverted {
            // (0,0) center at 0.5,0.5 is outside rect so should be selected after invert
            assert!(data[0] > 0.5);
            // (1,1) center at 1.5,1.5 is inside rect so should be unselected after invert
            assert!(data[5] < 0.5);
        } else {
            panic!("expected Mask");
        }
    }

    #[test]
    fn to_mask_ellipse() {
        let sel = Selection::Ellipse(Rect {
            x: 0.0,
            y: 0.0,
            width: 4.0,
            height: 4.0,
        });
        let mask = sel.to_mask(4, 4);
        if let Selection::Mask { data, .. } = &mask {
            // Center pixel (1,1) at 1.5,1.5 should be inside ellipse
            assert_eq!(data[5], 1.0);
        } else {
            panic!("expected Mask");
        }
    }

    #[test]
    fn to_mask_none_selects_everything() {
        let mask = Selection::None.to_mask(3, 3);
        if let Selection::Mask { data, .. } = &mask {
            assert!(data.iter().all(|&v| v == 1.0));
        } else {
            panic!("expected Mask");
        }
    }

    #[test]
    fn to_mask_returns_self_for_mask() {
        let sel = Selection::Mask {
            width: 2,
            height: 2,
            data: vec![0.5, 0.5, 0.5, 0.5],
        };
        let mask = sel.to_mask(2, 2);
        if let Selection::Mask { data, .. } = &mask {
            assert_eq!(data[0], 0.5);
        }
    }

    #[test]
    fn freeform_less_than_3_points() {
        let sel = Selection::Freeform {
            points: vec![Point { x: 0.0, y: 0.0 }, Point { x: 1.0, y: 1.0 }],
        };
        assert!(!sel.contains(Point { x: 0.5, y: 0.5 }));
    }
}
