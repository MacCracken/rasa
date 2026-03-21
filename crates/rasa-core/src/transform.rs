use serde::{Deserialize, Serialize};

use crate::geometry::Point;

/// 2D affine transform matrix (3x2).
///
/// Delegates to [`ranga::transform::Affine`] for the underlying math.
///
/// ```text
/// | a  c  tx |
/// | b  d  ty |
/// | 0  0  1  |
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Transform {
    inner: ranga::transform::Affine,
}

impl Transform {
    pub const IDENTITY: Self = Self {
        inner: ranga::transform::Affine::IDENTITY,
    };

    pub fn translate(tx: f64, ty: f64) -> Self {
        Self {
            inner: ranga::transform::Affine::translate(tx, ty),
        }
    }

    pub fn scale(sx: f64, sy: f64) -> Self {
        Self {
            inner: ranga::transform::Affine::scale(sx, sy),
        }
    }

    pub fn rotate(angle_rad: f64) -> Self {
        Self {
            inner: ranga::transform::Affine::rotate(angle_rad),
        }
    }

    /// Multiply two transforms: self * other (apply other first, then self).
    pub fn then(&self, other: &Transform) -> Self {
        Self {
            inner: self.inner.then(&other.inner),
        }
    }

    /// Apply this transform to a point.
    pub fn apply(&self, p: Point) -> Point {
        let (x, y) = self.inner.apply(p.x, p.y);
        Point { x, y }
    }

    /// Compute the inverse transform, if it exists.
    pub fn inverse(&self) -> Option<Self> {
        self.inner.inverse().map(|inv| Self { inner: inv })
    }

    pub fn is_identity(&self) -> bool {
        self.inner.is_identity()
    }

    // Field accessors for serialization compatibility
    pub fn a(&self) -> f64 {
        self.inner.a
    }
    pub fn b(&self) -> f64 {
        self.inner.b
    }
    pub fn c(&self) -> f64 {
        self.inner.c
    }
    pub fn d(&self) -> f64 {
        self.inner.d
    }
    pub fn tx(&self) -> f64 {
        self.inner.tx
    }
    pub fn ty(&self) -> f64 {
        self.inner.ty
    }
}

impl Default for Transform {
    fn default() -> Self {
        Self::IDENTITY
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::PI;

    fn approx_eq(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    #[test]
    fn identity_preserves_point() {
        let p = Point { x: 5.0, y: 7.0 };
        let result = Transform::IDENTITY.apply(p);
        assert!(approx_eq(result.x, 5.0));
        assert!(approx_eq(result.y, 7.0));
    }

    #[test]
    fn translate_moves_point() {
        let t = Transform::translate(10.0, 20.0);
        let result = t.apply(Point { x: 5.0, y: 5.0 });
        assert!(approx_eq(result.x, 15.0));
        assert!(approx_eq(result.y, 25.0));
    }

    #[test]
    fn scale_multiplies_point() {
        let t = Transform::scale(2.0, 3.0);
        let result = t.apply(Point { x: 5.0, y: 5.0 });
        assert!(approx_eq(result.x, 10.0));
        assert!(approx_eq(result.y, 15.0));
    }

    #[test]
    fn rotate_90() {
        let t = Transform::rotate(PI / 2.0);
        let result = t.apply(Point { x: 1.0, y: 0.0 });
        assert!(approx_eq(result.x, 0.0));
        assert!(approx_eq(result.y, 1.0));
    }

    #[test]
    fn then_composes() {
        let s = Transform::scale(2.0, 2.0);
        let t = Transform::translate(10.0, 10.0);
        let combined = t.then(&s);
        let result = combined.apply(Point { x: 5.0, y: 5.0 });
        assert!(approx_eq(result.x, 20.0));
        assert!(approx_eq(result.y, 20.0));
    }

    #[test]
    fn inverse_roundtrip() {
        let t = Transform::translate(10.0, 20.0)
            .then(&Transform::scale(2.0, 3.0))
            .then(&Transform::rotate(0.5));
        let inv = t.inverse().unwrap();
        let p = Point { x: 7.0, y: 13.0 };
        let transformed = t.apply(p);
        let back = inv.apply(transformed);
        assert!(approx_eq(back.x, p.x));
        assert!(approx_eq(back.y, p.y));
    }

    #[test]
    fn inverse_singular() {
        let t = Transform::scale(0.0, 1.0);
        assert!(t.inverse().is_none());
    }

    #[test]
    fn is_identity() {
        assert!(Transform::IDENTITY.is_identity());
        assert!(!Transform::translate(1.0, 0.0).is_identity());
    }
}
