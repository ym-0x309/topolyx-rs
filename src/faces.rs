//! Face traversal and naive triangulation, built on top of `topology.face_offsets`
//! (SPECIFICATION.md section 4, `face_offsets`).

use crate::error::TopolyxError;
use crate::file::{Mesh, Object};

/// One face's corners: slices into a mesh's `corner_vertices`/`corner_edges` arrays, in
/// polygon traversal order (SPECIFICATION.md section 4, `winding`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Face<'a> {
    pub vertices: &'a [u32],
    pub edges: &'a [u32],
}

impl<'a> Face<'a> {
    /// Naive fan triangulation: `(vertices[0], vertices[i], vertices[i + 1])` for each `i` in
    /// `1..vertices.len() - 1`. Yields nothing for a face with fewer than 3 vertices (not a
    /// valid face per SPECIFICATION.md section 4, but reachable without calling `validate()`
    /// first).
    ///
    /// This is only correct for convex, planar faces. SPECIFICATION.md section 4
    /// (`face_offsets`) explicitly leaves triangulation of non-planar/concave n-gons out of
    /// the format's scope, so this is a best-effort convenience rather than a
    /// spec-guaranteed-correct triangulation.
    pub fn triangulate_fan(&self) -> impl Iterator<Item = [u32; 3]> + 'a + use<'a> {
        // `rest` is empty whenever `vertices.len() < 3`, so the closure below — and its
        // `first.unwrap()` — is never invoked in that case.
        let first = self.vertices.first().copied();
        let rest = self.vertices.get(1..).unwrap_or(&[]);
        rest.windows(2).map(move |w| [first.unwrap(), w[0], w[1]])
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

    /// A flat triangle-index buffer for the whole mesh (3 indices per triangle), naive
    /// fan-triangulating every face — see [`Face::triangulate_fan`] for the non-planar/concave
    /// caveat. Ready to hand to a GPU as an index buffer.
    pub fn triangulate_fan_indices(&self, bin: &[u8]) -> Result<Vec<u32>, TopolyxError> {
        Ok(self.faces(bin)?.iter().flat_map(|f| f.triangulate_fan()).flatten().collect())
    }

    /// `topology.positions()` reindexed via `topology.corner_vertices()`: one position per face
    /// corner (length `corners`) instead of one per vertex (length `vertices`), so it lines up
    /// with any CORNER-domain attribute (e.g. `NORMAL`, UVs). Useful for interleaving positions
    /// with corner-domain attributes into a single per-corner GPU vertex buffer.
    pub fn corner_positions(&self, bin: &[u8]) -> Result<Vec<[f32; 3]>, TopolyxError> {
        let positions = self.topology.positions(bin)?;
        let corner_vertices = self.topology.corner_vertices(bin)?;
        expand_by_index(&positions, &corner_vertices, "topology.positions")
    }

    /// [`Mesh::corner_positions`], with `object.transform` applied first via
    /// [`Mesh::world_positions`] — a per-corner position buffer ready for rendering in world
    /// space.
    pub fn world_corner_positions(&self, object: &Object, bin: &[u8]) -> Result<Vec<[f32; 3]>, TopolyxError> {
        let positions = self.world_positions(object, bin)?;
        let corner_vertices = self.topology.corner_vertices(bin)?;
        expand_by_index(&positions, &corner_vertices, "topology.positions")
    }
}

/// Reindexes `values` by `indices` (e.g. `corner_vertices`), one output element per index.
///
/// `indices` comes straight from the file and is not guaranteed in range without calling
/// `TopolyxFile::validate` first, so this checks bounds explicitly rather than indexing
/// directly (which could panic on a malformed file).
fn expand_by_index<T: Copy>(values: &[T], indices: &[u32], field: &'static str) -> Result<Vec<T>, TopolyxError> {
    indices
        .iter()
        .enumerate()
        .map(|(corner, &i)| {
            values.get(i as usize).copied().ok_or(TopolyxError::CornerIndexOutOfRange {
                field,
                corner,
                index: i as usize,
                length: values.len(),
            })
        })
        .collect()
}
