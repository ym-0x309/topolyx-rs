//! World-space application of `object.transform` to topology/attribute values, per
//! SPECIFICATION.md section 4, "Object Transform Application Rules".
//!
//! `ROTATION` is not handled here: converting a `ROTATION`-semantic attribute to world space
//! requires extracting the transform's pure-rotation component, which in the general case
//! (when the transform's linear part includes scale/shear) needs a decomposition algorithm
//! (e.g. polar decomposition) this crate does not implement yet. Calling
//! [`Attribute::world_values`] on a `ROTATION`-semantic attribute returns
//! [`TopolyxError::UnsupportedTransformSemantic`]. This is a known gap, tracked for a future
//! version.

use crate::error::TopolyxError;
use crate::file::{Attribute, Mesh, Object, Semantic};
use crate::grouped::AttributeValues;

/// Row-major 3x3 linear part of a column-major 4x4 `transform`
/// (`L[row][col] == transform[col * 4 + row]`).
type Mat3 = [[f32; 3]; 3];

pub(crate) fn linear_part(transform: &[f32; 16]) -> Mat3 {
    [
        [transform[0], transform[4], transform[8]],
        [transform[1], transform[5], transform[9]],
        [transform[2], transform[6], transform[10]],
    ]
}

fn translation_part(transform: &[f32; 16]) -> [f32; 3] {
    [transform[12], transform[13], transform[14]]
}

/// Determinant of a 3x3 matrix (SPECIFICATION.md section 5, "Transform Validity").
pub(crate) fn determinant(m: &Mat3) -> f32 {
    let [[a, b, c], [d, e, f], [g, h, i]] = *m;
    a * (e * i - f * h) - b * (d * i - f * g) + c * (d * h - e * g)
}

fn transpose(m: &Mat3) -> Mat3 {
    [
        [m[0][0], m[1][0], m[2][0]],
        [m[0][1], m[1][1], m[2][1]],
        [m[0][2], m[1][2], m[2][2]],
    ]
}

/// Inverse of a non-singular 3x3 matrix, via the adjugate/cofactor method. `None` if `m` is
/// singular (determinant is 0).
fn inverse(m: &Mat3) -> Option<Mat3> {
    let det = determinant(m);
    if det == 0.0 {
        return None;
    }
    let inv_det = 1.0 / det;
    let [[a, b, c], [d, e, f], [g, h, i]] = *m;
    // cofactor_rows[i][j] is the (i, j) cofactor of m; the inverse is the transpose of the
    // cofactor matrix (the adjugate), scaled by 1/det.
    let cofactor_rows = [
        [e * i - f * h, -(d * i - f * g), d * h - e * g],
        [-(b * i - c * h), a * i - c * g, -(a * h - b * g)],
        [b * f - c * e, -(a * f - c * d), a * e - b * d],
    ];
    Some([
        [cofactor_rows[0][0] * inv_det, cofactor_rows[1][0] * inv_det, cofactor_rows[2][0] * inv_det],
        [cofactor_rows[0][1] * inv_det, cofactor_rows[1][1] * inv_det, cofactor_rows[2][1] * inv_det],
        [cofactor_rows[0][2] * inv_det, cofactor_rows[1][2] * inv_det, cofactor_rows[2][2] * inv_det],
    ])
}

fn mul_vec3(m: &Mat3, v: [f32; 3]) -> [f32; 3] {
    [
        m[0][0] * v[0] + m[0][1] * v[1] + m[0][2] * v[2],
        m[1][0] * v[0] + m[1][1] * v[1] + m[1][2] * v[2],
        m[2][0] * v[0] + m[2][1] * v[1] + m[2][2] * v[2],
    ]
}

fn normalize3(v: [f32; 3]) -> [f32; 3] {
    let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if len == 0.0 { v } else { [v[0] / len, v[1] / len, v[2] / len] }
}

fn apply_position(l: &Mat3, t: [f32; 3], p: [f32; 3]) -> [f32; 3] {
    let lp = mul_vec3(l, p);
    [lp[0] + t[0], lp[1] + t[1], lp[2] + t[2]]
}

fn apply_direction(l: &Mat3, v: [f32; 3]) -> [f32; 3] {
    mul_vec3(l, v)
}

fn apply_normal(inverse_transpose_linear: &Mat3, n: [f32; 3]) -> [f32; 3] {
    normalize3(mul_vec3(inverse_transpose_linear, n))
}

fn apply_tangent(l: &Mat3, det_negative: bool, t: [f32; 4]) -> [f32; 4] {
    let xyz = normalize3(apply_direction(l, [t[0], t[1], t[2]]));
    let w = if det_negative { -t[3] } else { t[3] };
    [xyz[0], xyz[1], xyz[2], w]
}

/// Values derived from `object.transform` once and reused across every attribute of a mesh, so
/// `inverse(L)` (needed for `NORMAL`) is computed at most once per `world_*` call.
struct TransformContext {
    linear: Mat3,
    translation: [f32; 3],
    inverse_transpose_linear: Mat3,
    det_negative: bool,
}

impl TransformContext {
    fn new(object: &Object) -> Result<Self, TopolyxError> {
        let linear = linear_part(&object.transform);
        let inverse_linear = inverse(&linear).ok_or(TopolyxError::SingularObjectTransform)?;
        Ok(Self {
            det_negative: determinant(&linear) < 0.0,
            linear,
            translation: translation_part(&object.transform),
            inverse_transpose_linear: transpose(&inverse_linear),
        })
    }
}

impl Mesh {
    /// World-space vertex positions: `topology.positions()` with `object.transform` applied
    /// (`L * p + t`; SPECIFICATION.md section 4, `positions`/`POSITION`).
    pub fn world_positions(&self, object: &Object, bin: &[u8]) -> Result<Vec<[f32; 3]>, TopolyxError> {
        let ctx = TransformContext::new(object)?;
        Ok(self
            .topology
            .positions(bin)?
            .into_iter()
            .map(|p| apply_position(&ctx.linear, ctx.translation, p))
            .collect())
    }
}

impl Attribute {
    /// This attribute's values (see [`Attribute::values`]), converted to world space via
    /// `object.transform` according to this attribute's `semantic`
    /// (SPECIFICATION.md section 4, "Object Transform Application Rules").
    ///
    /// `ROTATION` is not supported yet (see the module docs) and always returns
    /// [`TopolyxError::UnsupportedTransformSemantic`].
    pub fn world_values(&self, object: &Object, bin: &[u8]) -> Result<AttributeValues, TopolyxError> {
        if self.semantic == Semantic::Rotation {
            return Err(TopolyxError::UnsupportedTransformSemantic(self.semantic));
        }

        let values = self.values(bin)?;
        let needs_transform = matches!(
            self.semantic,
            Semantic::Position | Semantic::Direction | Semantic::Normal | Semantic::Tangent
        );
        if !needs_transform {
            // COLOR and NONE are not transformed (SPECIFICATION.md section 4, transform table).
            return Ok(values);
        }

        let ctx = TransformContext::new(object)?;
        Ok(match values {
            AttributeValues::Position(v) => AttributeValues::Position(
                v.into_iter().map(|p| apply_position(&ctx.linear, ctx.translation, p)).collect(),
            ),
            AttributeValues::Direction(v) => {
                AttributeValues::Direction(v.into_iter().map(|d| apply_direction(&ctx.linear, d)).collect())
            }
            AttributeValues::Normal(v) => AttributeValues::Normal(
                v.into_iter().map(|n| apply_normal(&ctx.inverse_transpose_linear, n)).collect(),
            ),
            AttributeValues::Tangent(v) => AttributeValues::Tangent(
                v.into_iter().map(|t| apply_tangent(&ctx.linear, ctx.det_negative, t)).collect(),
            ),
            other => other,
        })
    }
}
