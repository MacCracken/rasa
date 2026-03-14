use serde::{Deserialize, Serialize};

use crate::geometry::Point;

/// 2D affine transform matrix (3x2).
///
/// ```text
/// | a  c  tx |
/// | b  d  ty |
/// | 0  0  1  |
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Transform {
    pub a: f64,
    pub b: f64,
    pub c: f64,
    pub d: f64,
    pub tx: f64,
    pub ty: f64,
}

impl Transform {
    pub const IDENTITY: Self = Self {
        a: 1.0,
        b: 0.0,
        c: 0.0,
        d: 1.0,
        tx: 0.0,
        ty: 0.0,
    };

    pub fn translate(tx: f64, ty: f64) -> Self {
        Self {
            a: 1.0,
            b: 0.0,
            c: 0.0,
            d: 1.0,
            tx,
            ty,
        }
    }

    pub fn scale(sx: f64, sy: f64) -> Self {
        Self {
            a: sx,
            b: 0.0,
            c: 0.0,
            d: sy,
            tx: 0.0,
            ty: 0.0,
        }
    }

    pub fn rotate(angle_rad: f64) -> Self {
        let cos = angle_rad.cos();
        let sin = angle_rad.sin();
        Self {
            a: cos,
            b: sin,
            c: -sin,
            d: cos,
            tx: 0.0,
            ty: 0.0,
        }
    }

    /// Multiply two transforms: self * other (apply other first, then self).
    pub fn then(&self, other: &Transform) -> Self {
        Self {
            a: self.a * other.a + self.c * other.b,
            b: self.b * other.a + self.d * other.b,
            c: self.a * other.c + self.c * other.d,
            d: self.b * other.c + self.d * other.d,
            tx: self.a * other.tx + self.c * other.ty + self.tx,
            ty: self.b * other.tx + self.d * other.ty + self.ty,
        }
    }

    /// Apply this transform to a point.
    pub fn apply(&self, p: Point) -> Point {
        Point {
            x: self.a * p.x + self.c * p.y + self.tx,
            y: self.b * p.x + self.d * p.y + self.ty,
        }
    }

    /// Compute the inverse transform, if it exists.
    pub fn inverse(&self) -> Option<Self> {
        let det = self.a * self.d - self.b * self.c;
        if det.abs() < 1e-12 {
            return None;
        }
        let inv_det = 1.0 / det;
        Some(Self {
            a: self.d * inv_det,
            b: -self.b * inv_det,
            c: -self.c * inv_det,
            d: self.a * inv_det,
            tx: (self.c * self.ty - self.d * self.tx) * inv_det,
            ty: (self.b * self.tx - self.a * self.ty) * inv_det,
        })
    }

    pub fn is_identity(&self) -> bool {
        (self.a - 1.0).abs() < 1e-9
            && self.b.abs() < 1e-9
            && self.c.abs() < 1e-9
            && (self.d - 1.0).abs() < 1e-9
            && self.tx.abs() < 1e-9
            && self.ty.abs() < 1e-9
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
        // Scale then translate: point (5,5) -> (10,10) -> (20,20)
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
