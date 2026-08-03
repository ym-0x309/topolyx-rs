//! Face traversal and naive triangulation, built on top of `topology.face_offsets`
//! (SPECIFICATION.md section 4, `face_offsets`).

use crate::error::TopolyxError;
use crate::file::Mesh;

/// One face's corners: slices into a mesh's `corner_vertices`/`corner_edges` arrays, in
/// polygon traversal order (SPECIFICATION.md section 4, `winding`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Face<'a> {
    pub vertices: &'a [u32],
    pub edges: &'a [u32],
}

impl<'a> Face<'a> {
    /// Naive fan triangulation: `(vertices[0], vertices[i], vertices[i + 1])` for each `i` in
    /// `1..vertices.len() - 1`.
    ///
    /// This is only correct for convex, planar faces. SPECIFICATION.md section 4
    /// (`face_offsets`) explicitly leaves triangulation of non-planar/concave n-gons out of
    /// the format's scope, so this is a best-effort convenience rather than a
    /// spec-guaranteed-correct triangulation.
    pub fn triangulate_fan(&self) -> impl Iterator<Item = [u32; 3]> + 'a {
        let first = self.vertices[0];
        self.vertices[1..].windows(2).map(move |w| [first, w[0], w[1]])
    }
}

/// Owned per-face corner data for a whole mesh, produced by [`Mesh::faces`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FaceCorners {
    face_offsets: Vec<u32>,
    corner_vertices: Vec<u32>,
    corner_edges: Vec<u32>,
}

impl FaceCorners {
    /// Number of faces (`face_offsets.len() - 1`).
    pub fn len(&self) -> usize {
        self.face_offsets.len().saturating_sub(1)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The `index`-th face's corners, or `None` if `index >= self.len()`.
    pub fn face(&self, index: usize) -> Option<Face<'_>> {
        let start = *self.face_offsets.get(index)? as usize;
        let end = *self.face_offsets.get(index + 1)? as usize;
        Some(Face {
            vertices: &self.corner_vertices[start..end],
            edges: &self.corner_edges[start..end],
        })
    }

    pub fn iter(&self) -> impl Iterator<Item = Face<'_>> {
        (0..self.len()).map(move |i| self.face(i).expect("index within FaceCorners::len()"))
    }
}

impl Mesh {
    /// Groups this mesh's `corner_vertices`/`corner_edges` by face, using `topology.face_offsets`.
    pub fn faces(&self, bin: &[u8]) -> Result<FaceCorners, TopolyxError> {
        Ok(FaceCorners {
            face_offsets: self.topology.face_offsets(bin)?,
            corner_vertices: self.topology.corner_vertices(bin)?,
            corner_edges: self.topology.corner_edges(bin)?,
        })
    }
}
