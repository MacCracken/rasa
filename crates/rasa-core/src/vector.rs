use serde::{Deserialize, Serialize};

use crate::color::Color;
use crate::geometry::Point;

/// A complete vector path with stroke and fill styling.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorPath {
    pub segments: Vec<PathSegment>,
    pub closed: bool,
    pub fill: Option<FillStyle>,
    pub stroke: Option<StrokeStyle>,
}

/// A segment of a vector path.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum PathSegment {
    MoveTo(Point),
    LineTo(Point),
    QuadTo {
        ctrl: Point,
        end: Point,
    },
    CubicTo {
        ctrl1: Point,
        ctrl2: Point,
        end: Point,
    },
}

/// Fill style for a closed path.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FillStyle {
    Solid(Color),
    LinearGradient {
        start: Point,
        end: Point,
        color_start: Color,
        color_end: Color,
    },
    RadialGradient {
        center: Point,
        radius: f64,
        color_center: Color,
        color_edge: Color,
    },
}

/// Stroke style for a path outline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrokeStyle {
    pub color: Color,
    pub width: f64,
    pub cap: LineCap,
    pub join: LineJoin,
}

/// Line cap style for stroke endpoints.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum LineCap {
    #[default]
    Butt,
    Round,
    Square,
}

/// Line join style for stroke corners.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum LineJoin {
    #[default]
    Miter,
    Round,
    Bevel,
}

/// Data payload for a vector layer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorData {
    pub paths: Vec<VectorPath>,
}

impl VectorData {
    pub fn new() -> Self {
        Self { paths: Vec::new() }
    }

    pub fn add_path(&mut self, path: VectorPath) {
        self.paths.push(path);
    }
}

impl Default for VectorData {
    fn default() -> Self {
        Self::new()
    }
}

impl VectorPath {
    /// Create a rectangle path.
    pub fn rect(
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        fill: Option<FillStyle>,
        stroke: Option<StrokeStyle>,
    ) -> Self {
        Self {
            segments: vec![
                PathSegment::MoveTo(Point { x, y }),
                PathSegment::LineTo(Point { x: x + w, y }),
                PathSegment::LineTo(Point { x: x + w, y: y + h }),
                PathSegment::LineTo(Point { x, y: y + h }),
            ],
            closed: true,
            fill,
            stroke,
        }
    }

    /// Create an ellipse path approximated with four cubic bezier curves.
    ///
    /// Uses the standard four-arc approximation with the kappa constant
    /// (4/3 * (sqrt(2) - 1)) for near-perfect circular arcs.
    pub fn ellipse(
        cx: f64,
        cy: f64,
        rx: f64,
        ry: f64,
        fill: Option<FillStyle>,
        stroke: Option<StrokeStyle>,
    ) -> Self {
        // Kappa: control point offset for a quarter circle.
        const KAPPA: f64 = 0.552_284_749_830_793_4;
        let kx = rx * KAPPA;
        let ky = ry * KAPPA;

        Self {
            segments: vec![
                // Start at rightmost point
                PathSegment::MoveTo(Point { x: cx + rx, y: cy }),
                // Top-right quarter
                PathSegment::CubicTo {
                    ctrl1: Point {
                        x: cx + rx,
                        y: cy - ky,
                    },
                    ctrl2: Point {
                        x: cx + kx,
                        y: cy - ry,
                    },
                    end: Point { x: cx, y: cy - ry },
                },
                // Top-left quarter
                PathSegment::CubicTo {
                    ctrl1: Point {
                        x: cx - kx,
                        y: cy - ry,
                    },
                    ctrl2: Point {
                        x: cx - rx,
                        y: cy - ky,
                    },
                    end: Point { x: cx - rx, y: cy },
                },
                // Bottom-left quarter
                PathSegment::CubicTo {
                    ctrl1: Point {
                        x: cx - rx,
                        y: cy + ky,
                    },
                    ctrl2: Point {
                        x: cx - kx,
                        y: cy + ry,
                    },
                    end: Point { x: cx, y: cy + ry },
                },
                // Bottom-right quarter
                PathSegment::CubicTo {
                    ctrl1: Point {
                        x: cx + kx,
                        y: cy + ry,
                    },
                    ctrl2: Point {
                        x: cx + rx,
                        y: cy + ky,
                    },
                    end: Point { x: cx + rx, y: cy },
                },
            ],
            closed: true,
            fill,
            stroke,
        }
    }

    /// Create a straight line.
    pub fn line(x1: f64, y1: f64, x2: f64, y2: f64, stroke: StrokeStyle) -> Self {
        Self {
            segments: vec![
                PathSegment::MoveTo(Point { x: x1, y: y1 }),
                PathSegment::LineTo(Point { x: x2, y: y2 }),
            ],
            closed: false,
            fill: None,
            stroke: Some(stroke),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vector_data_new_empty() {
        let data = VectorData::new();
        assert!(data.paths.is_empty());
    }

    #[test]
    fn vector_data_add_path() {
        let mut data = VectorData::new();
        let path = VectorPath::rect(0.0, 0.0, 10.0, 10.0, None, None);
        data.add_path(path);
        assert_eq!(data.paths.len(), 1);
    }

    #[test]
    fn vector_path_rect() {
        let path = VectorPath::rect(5.0, 10.0, 20.0, 30.0, None, None);
        assert!(path.closed);
        // MoveTo + 3 LineTo = 4 segments
        assert_eq!(path.segments.len(), 4);
        assert!(matches!(path.segments[0], PathSegment::MoveTo(p) if p.x == 5.0 && p.y == 10.0));
    }

    #[test]
    fn vector_path_ellipse() {
        let path = VectorPath::ellipse(50.0, 50.0, 25.0, 25.0, None, None);
        assert!(path.closed);
        // MoveTo + 4 CubicTo = 5 segments
        assert_eq!(path.segments.len(), 5);
        assert!(matches!(path.segments[0], PathSegment::MoveTo(_)));
        assert!(matches!(path.segments[1], PathSegment::CubicTo { .. }));
    }

    #[test]
    fn vector_path_line() {
        let stroke = StrokeStyle {
            color: Color::BLACK,
            width: 2.0,
            cap: LineCap::default(),
            join: LineJoin::default(),
        };
        let path = VectorPath::line(0.0, 0.0, 100.0, 100.0, stroke);
        assert!(!path.closed);
        assert!(path.fill.is_none());
        assert!(path.stroke.is_some());
        assert_eq!(path.segments.len(), 2);
    }

    #[test]
    fn path_segment_variants() {
        let _move = PathSegment::MoveTo(Point { x: 0.0, y: 0.0 });
        let _line = PathSegment::LineTo(Point { x: 1.0, y: 1.0 });
        let _quad = PathSegment::QuadTo {
            ctrl: Point { x: 0.5, y: 1.0 },
            end: Point { x: 1.0, y: 0.0 },
        };
        let _cubic = PathSegment::CubicTo {
            ctrl1: Point { x: 0.25, y: 0.75 },
            ctrl2: Point { x: 0.75, y: 0.75 },
            end: Point { x: 1.0, y: 0.0 },
        };
        // All four variants are constructible without panic.
    }

    #[test]
    fn fill_style_solid() {
        let fill = FillStyle::Solid(Color::WHITE);
        if let FillStyle::Solid(c) = fill {
            assert_eq!(c, Color::WHITE);
        } else {
            panic!("expected Solid");
        }
    }

    #[test]
    fn fill_style_linear_gradient() {
        let fill = FillStyle::LinearGradient {
            start: Point { x: 0.0, y: 0.0 },
            end: Point { x: 10.0, y: 0.0 },
            color_start: Color::BLACK,
            color_end: Color::WHITE,
        };
        assert!(matches!(fill, FillStyle::LinearGradient { .. }));
    }

    #[test]
    fn fill_style_radial_gradient() {
        let fill = FillStyle::RadialGradient {
            center: Point { x: 5.0, y: 5.0 },
            radius: 10.0,
            color_center: Color::WHITE,
            color_edge: Color::BLACK,
        };
        assert!(matches!(fill, FillStyle::RadialGradient { .. }));
    }

    #[test]
    fn stroke_style_defaults() {
        let stroke = StrokeStyle {
            color: Color::BLACK,
            width: 1.0,
            cap: LineCap::default(),
            join: LineJoin::default(),
        };
        assert_eq!(stroke.color, Color::BLACK);
        assert_eq!(stroke.width, 1.0);
        assert_eq!(stroke.cap, LineCap::Butt);
        assert_eq!(stroke.join, LineJoin::Miter);
    }

    #[test]
    fn line_cap_default_is_butt() {
        assert_eq!(LineCap::default(), LineCap::Butt);
    }

    #[test]
    fn line_join_default_is_miter() {
        assert_eq!(LineJoin::default(), LineJoin::Miter);
    }

    #[test]
    fn vector_data_serde_roundtrip() {
        let mut data = VectorData::new();
        data.add_path(VectorPath::rect(
            0.0,
            0.0,
            100.0,
            50.0,
            Some(FillStyle::Solid(Color::WHITE)),
            Some(StrokeStyle {
                color: Color::BLACK,
                width: 2.0,
                cap: LineCap::Round,
                join: LineJoin::Bevel,
            }),
        ));
        data.add_path(VectorPath::ellipse(
            50.0,
            50.0,
            20.0,
            30.0,
            Some(FillStyle::Solid(Color::BLACK)),
            None,
        ));

        let json = serde_json::to_string(&data).expect("serialize failed");
        let data2: VectorData = serde_json::from_str(&json).expect("deserialize failed");

        assert_eq!(data2.paths.len(), 2);
        assert!(data2.paths[0].closed);
        assert_eq!(data2.paths[0].segments.len(), 4);
        assert!(data2.paths[1].closed);
        assert_eq!(data2.paths[1].segments.len(), 5);
    }
}
