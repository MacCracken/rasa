use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Size {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl Rect {
    pub fn contains(&self, point: Point) -> bool {
        point.x >= self.x
            && point.x <= self.x + self.width
            && point.y >= self.y
            && point.y <= self.y + self.height
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn point_creation() {
        let p = Point { x: 3.0, y: 4.0 };
        assert_eq!(p.x, 3.0);
        assert_eq!(p.y, 4.0);
    }

    #[test]
    fn point_clone_and_copy() {
        let p = Point { x: 1.0, y: 2.0 };
        let p2 = p;
        assert_eq!(p, p2);
    }

    #[test]
    fn size_creation() {
        let s = Size {
            width: 1920,
            height: 1080,
        };
        assert_eq!(s.width, 1920);
        assert_eq!(s.height, 1080);
    }

    #[test]
    fn size_clone_and_copy() {
        let s = Size {
            width: 10,
            height: 20,
        };
        let s2 = s;
        assert_eq!(s, s2);
    }

    #[test]
    fn rect_creation() {
        let r = Rect {
            x: 5.0,
            y: 10.0,
            width: 100.0,
            height: 50.0,
        };
        assert_eq!(r.x, 5.0);
        assert_eq!(r.y, 10.0);
        assert_eq!(r.width, 100.0);
        assert_eq!(r.height, 50.0);
    }

    #[test]
    fn rect_contains_inside() {
        let r = Rect {
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 100.0,
        };
        assert!(r.contains(Point { x: 50.0, y: 50.0 }));
    }

    #[test]
    fn rect_contains_on_edges() {
        let r = Rect {
            x: 10.0,
            y: 10.0,
            width: 20.0,
            height: 20.0,
        };
        // On all four edges
        assert!(r.contains(Point { x: 10.0, y: 15.0 })); // left edge
        assert!(r.contains(Point { x: 30.0, y: 15.0 })); // right edge
        assert!(r.contains(Point { x: 15.0, y: 10.0 })); // top edge
        assert!(r.contains(Point { x: 15.0, y: 30.0 })); // bottom edge
    }

    #[test]
    fn rect_contains_corners() {
        let r = Rect {
            x: 0.0,
            y: 0.0,
            width: 10.0,
            height: 10.0,
        };
        assert!(r.contains(Point { x: 0.0, y: 0.0 }));
        assert!(r.contains(Point { x: 10.0, y: 10.0 }));
        assert!(r.contains(Point { x: 0.0, y: 10.0 }));
        assert!(r.contains(Point { x: 10.0, y: 0.0 }));
    }

    #[test]
    fn rect_not_contains_outside() {
        let r = Rect {
            x: 10.0,
            y: 10.0,
            width: 20.0,
            height: 20.0,
        };
        assert!(!r.contains(Point { x: 5.0, y: 15.0 }));  // left
        assert!(!r.contains(Point { x: 35.0, y: 15.0 })); // right
        assert!(!r.contains(Point { x: 15.0, y: 5.0 }));  // above
        assert!(!r.contains(Point { x: 15.0, y: 35.0 })); // below
    }

    #[test]
    fn rect_zero_size() {
        let r = Rect {
            x: 5.0,
            y: 5.0,
            width: 0.0,
            height: 0.0,
        };
        // Only the exact point should be contained
        assert!(r.contains(Point { x: 5.0, y: 5.0 }));
        assert!(!r.contains(Point { x: 5.1, y: 5.0 }));
    }

    #[test]
    fn point_debug_format() {
        let p = Point { x: 1.5, y: 2.5 };
        let debug = format!("{:?}", p);
        assert!(debug.contains("Point"));
        assert!(debug.contains("1.5"));
    }
}
