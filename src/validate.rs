//! Full spec section 5 ("Validity Conditions") validation pass over an already-parsed
//! [`TopolyxFile`].

use std::collections::{HashMap, HashSet};

use crate::error::TopolyxError;
use crate::file::{
    Attribute, ComponentType, CoordinateSystem, DataDescriptor, Domain, ElementCounts, Mesh,
    Object, Semantic, Topology, TopolyxFile,
};

impl TopolyxFile {
    /// Runs the full spec section 5 validity-condition pass over this file.
    ///
    /// `bin` must be the BIN chunk bytes returned alongside this `TopolyxFile` by
    /// [`crate::read_topolyx`]/[`crate::read_topolyx_from_data`]. This is not run
    /// automatically by those functions — call it explicitly once the full spec-validity
    /// guarantee is needed (e.g. before trusting the file's topology to be internally
    /// consistent).
    pub fn validate(&self, bin: &[u8]) -> Result<(), TopolyxError> {
        validate_coordinate_system(&self.coordinate_system)?;
        validate_object_names(&self.objects)?;
        validate_mesh_names(&self.meshes)?;
        validate_object_mesh_indices(&self.objects, self.meshes.len())?;

        for (i, object) in self.objects.iter().enumerate() {
            validate_transform(i, &object.transform)?;
        }

        for (i, mesh) in self.meshes.iter().enumerate() {
            validate_mesh(i, mesh, bin)?;
        }

        validate_bin_tail_padding(&self.meshes, bin)?;

        Ok(())
    }
}

fn validate_mesh(mesh_index: usize, mesh: &Mesh, bin: &[u8]) -> Result<(), TopolyxError> {
    validate_topology_shape(mesh_index, &mesh.topology)?;
    validate_attribute_names(mesh_index, &mesh.attributes)?;
    validate_attribute_semantics(mesh_index, &mesh.attributes)?;
    validate_element_counts(mesh_index, mesh)?;

    // Decode every descriptor not otherwise touched below so existing data.rs errors
    // (e.g. ByteLengthMismatch) are surfaced by validate() too, not just section 5 rules.
    mesh.topology.positions.extract(bin)?;
    for attribute in &mesh.attributes {
        attribute.data.extract(bin)?;
    }

    validate_mesh_topology_values(mesh_index, mesh, bin)
}

fn validate_mesh_topology_values(
    mesh_index: usize,
    mesh: &Mesh,
    bin: &[u8],
) -> Result<(), TopolyxError> {
    let counts = &mesh.element_counts;

    let edges_data = mesh.topology.edges.extract(bin)?;
    let edges = edges_data
        .as_u32()
        .expect("topology.edges checked U32 by validate_topology_shape");

    let corner_vertices_data = mesh.topology.corner_vertices.extract(bin)?;
    let corner_vertices = corner_vertices_data
        .as_u32()
        .expect("topology.corner_vertices checked U32 by validate_topology_shape");

    let corner_edges_data = mesh.topology.corner_edges.extract(bin)?;
    let corner_edges = corner_edges_data
        .as_u32()
        .expect("topology.corner_edges checked U32 by validate_topology_shape");

    let face_offsets_data = mesh.topology.face_offsets.extract(bin)?;
    let face_offsets = face_offsets_data
        .as_u32()
        .expect("topology.face_offsets checked U32 by validate_topology_shape");

    validate_edges(mesh_index, edges, counts.vertices)?;

    for &v in corner_vertices {
        check_index(mesh_index, "corner_vertices", v, counts.vertices)?;
    }
    for &e in corner_edges {
        check_index(mesh_index, "corner_edges", e, counts.edges)?;
    }

    // Runs before the general structural check below so an all-zero-counts mesh with a bad
    // face_offsets value gets the more specific EmptyMeshFaceOffsets error rather than the
    // generic InvalidFaceOffsets one. Safe to check first: when the mesh is fully empty,
    // faces == 0 so face_offsets has exactly one element, and nothing downstream slices by it
    // yet regardless of which of these two checks runs first.
    validate_empty_mesh(mesh_index, counts, face_offsets)?;

    // face_offsets' structural shape (starts at 0, non-decreasing, ends at corners) must be
    // confirmed before slicing corner_vertices/corner_edges by it below, or a malformed file
    // could make that slicing panic instead of returning a proper error.
    validate_face_offsets_structure(mesh_index, face_offsets, counts.corners)?;
    validate_corner_edge_consistency(mesh_index, face_offsets, corner_vertices, corner_edges, edges)
}

fn validate_coordinate_system(cs: &CoordinateSystem) -> Result<(), TopolyxError> {
    check_fixed_field("up_axis", "+Z", &cs.up_axis)?;
    check_fixed_field("forward_axis", "+Y", &cs.forward_axis)?;
    check_fixed_field("handedness", "RIGHT", &cs.handedness)?;
    check_fixed_field("winding", "CCW", &cs.winding)?;

    if !(cs.meters_per_unit.is_finite() && cs.meters_per_unit > 0.0) {
        return Err(TopolyxError::InvalidMetersPerUnit(cs.meters_per_unit));
    }

    Ok(())
}

fn check_fixed_field(
    field: &'static str,
    expected: &'static str,
    found: &str,
) -> Result<(), TopolyxError> {
    if found != expected {
        return Err(TopolyxError::InvalidCoordinateSystemField {
            field,
            expected,
            found: found.to_string(),
        });
    }
    Ok(())
}

fn validate_object_names(objects: &[Object]) -> Result<(), TopolyxError> {
    let mut seen = HashSet::new();
    for object in objects {
        if object.name.is_empty() {
            return Err(TopolyxError::EmptyName("object.name"));
        }
        if !seen.insert(object.name.as_str()) {
            return Err(TopolyxError::DuplicateName {
                kind: "object.name",
                name: object.name.clone(),
            });
        }
    }
    Ok(())
}

fn validate_mesh_names(meshes: &[Mesh]) -> Result<(), TopolyxError> {
    let mut seen = HashSet::new();
    for mesh in meshes {
        if mesh.name.is_empty() {
            return Err(TopolyxError::EmptyName("mesh.name"));
        }
        if !seen.insert(mesh.name.as_str()) {
            return Err(TopolyxError::DuplicateName {
                kind: "mesh.name",
                name: mesh.name.clone(),
            });
        }
    }
    Ok(())
}

fn validate_attribute_names(mesh_index: usize, attributes: &[Attribute]) -> Result<(), TopolyxError> {
    let mut seen = HashSet::new();
    for attribute in attributes {
        if attribute.name.is_empty() {
            return Err(TopolyxError::EmptyAttributeName { mesh: mesh_index });
        }
        if !seen.insert(attribute.name.as_str()) {
            return Err(TopolyxError::DuplicateAttributeName {
                mesh: mesh_index,
                name: attribute.name.clone(),
            });
        }
    }
    Ok(())
}

fn validate_object_mesh_indices(objects: &[Object], mesh_count: usize) -> Result<(), TopolyxError> {
    for (i, object) in objects.iter().enumerate() {
        // Only "MESH" is currently defined by the spec; other (future) object types
        // reference unspecified arrays, so their `index` is out of scope for this check.
        if object.object_type == "MESH" && object.index >= mesh_count {
            return Err(TopolyxError::ObjectMeshIndexOutOfRange {
                object: i,
                index: object.index,
                length: mesh_count,
            });
        }
    }
    Ok(())
}

fn validate_transform(object_index: usize, transform: &[f32; 16]) -> Result<(), TopolyxError> {
    if linear_determinant(transform) == 0.0 {
        return Err(TopolyxError::SingularTransform {
            object: object_index,
        });
    }
    Ok(())
}

/// Determinant of the 3x3 linear part `L` of a column-major 4x4 `transform`
/// (spec section 4, "Object Transform Application Rules"; `L[row][col] == transform[col*4+row]`).
fn linear_determinant(transform: &[f32; 16]) -> f32 {
    let (a, d, g) = (transform[0], transform[1], transform[2]); // column 0
    let (b, e, h) = (transform[4], transform[5], transform[6]); // column 1
    let (c, f, i) = (transform[8], transform[9], transform[10]); // column 2
    a * (e * i - f * h) - b * (d * i - f * g) + c * (d * h - e * g)
}

fn validate_topology_shape(mesh_index: usize, topology: &Topology) -> Result<(), TopolyxError> {
    check_shape(mesh_index, "positions", &topology.positions, ComponentType::F32, 3)?;
    check_shape(mesh_index, "edges", &topology.edges, ComponentType::U32, 2)?;
    check_shape(mesh_index, "corner_vertices", &topology.corner_vertices, ComponentType::U32, 1)?;
    check_shape(mesh_index, "corner_edges", &topology.corner_edges, ComponentType::U32, 1)?;
    check_shape(mesh_index, "face_offsets", &topology.face_offsets, ComponentType::U32, 1)?;
    Ok(())
}

fn check_shape(
    mesh: usize,
    field: &'static str,
    data: &DataDescriptor,
    expected_type: ComponentType,
    expected_count: u32,
) -> Result<(), TopolyxError> {
    if data.component_type != expected_type || data.component_count != expected_count {
        return Err(TopolyxError::ComponentShapeMismatch {
            mesh,
            field,
            expected_type,
            expected_count,
            found_type: data.component_type,
            found_count: data.component_count,
        });
    }
    Ok(())
}

fn validate_attribute_semantics(mesh_index: usize, attributes: &[Attribute]) -> Result<(), TopolyxError> {
    for attribute in attributes {
        let allowed: &[(ComponentType, u32)] = match attribute.semantic {
            Semantic::Position | Semantic::Direction | Semantic::Normal => &[(ComponentType::F32, 3)],
            Semantic::Rotation | Semantic::Tangent => &[(ComponentType::F32, 4)],
            Semantic::Color => &[(ComponentType::F32, 4), (ComponentType::U8, 4)],
            Semantic::None => continue,
        };

        let data = &attribute.data;
        let matches = allowed
            .iter()
            .any(|&(ty, count)| data.component_type == ty && data.component_count == count);
        if !matches {
            return Err(TopolyxError::SemanticShapeMismatch {
                mesh: mesh_index,
                attribute: attribute.name.clone(),
                semantic: attribute.semantic,
                found_type: data.component_type,
                found_count: data.component_count,
            });
        }
    }
    Ok(())
}

fn validate_element_counts(mesh_index: usize, mesh: &Mesh) -> Result<(), TopolyxError> {
    let counts = &mesh.element_counts;

    check_topology_count(mesh_index, "positions", counts.vertices, mesh.topology.positions.element_count)?;
    check_topology_count(mesh_index, "edges", counts.edges, mesh.topology.edges.element_count)?;
    check_topology_count(
        mesh_index,
        "corner_vertices",
        counts.corners,
        mesh.topology.corner_vertices.element_count,
    )?;
    check_topology_count(
        mesh_index,
        "corner_edges",
        counts.corners,
        mesh.topology.corner_edges.element_count,
    )?;
    check_topology_count(
        mesh_index,
        "face_offsets",
        counts.faces.saturating_add(1),
        mesh.topology.face_offsets.element_count,
    )?;

    for attribute in &mesh.attributes {
        let expected = domain_element_count(attribute.domain, counts);
        if attribute.data.element_count != expected {
            return Err(TopolyxError::AttributeElementCountMismatch {
                mesh: mesh_index,
                attribute: attribute.name.clone(),
                domain: attribute.domain,
                expected,
                found: attribute.data.element_count,
            });
        }
    }

    Ok(())
}

fn check_topology_count(
    mesh: usize,
    field: &'static str,
    expected: u32,
    found: u32,
) -> Result<(), TopolyxError> {
    if found != expected {
        return Err(TopolyxError::TopologyElementCountMismatch {
            mesh,
            field,
            expected,
            found,
        });
    }
    Ok(())
}

fn domain_element_count(domain: Domain, counts: &ElementCounts) -> u32 {
    match domain {
        Domain::Point => counts.vertices,
        Domain::Edge => counts.edges,
        Domain::Face => counts.faces,
        Domain::Corner => counts.corners,
    }
}

fn check_index(
    mesh: usize,
    field: &'static str,
    index: u32,
    length: u32,
) -> Result<(), TopolyxError> {
    if index >= length {
        return Err(TopolyxError::IndexOutOfRange {
            mesh,
            field,
            index: index as usize,
            length: length as usize,
        });
    }
    Ok(())
}

/// Checks for self-edges and duplicate edges, and that every vertex index is in range
/// (spec section 4 "edges" + section 5 "Index Range").
fn validate_edges(mesh_index: usize, edges: &[u32], vertex_count: u32) -> Result<(), TopolyxError> {
    let mut seen: HashMap<(u32, u32), u32> = HashMap::new();

    for (i, pair) in edges.chunks_exact(2).enumerate() {
        let (a, b) = (pair[0], pair[1]);
        let edge_index = i as u32;

        check_index(mesh_index, "edges", a, vertex_count)?;
        check_index(mesh_index, "edges", b, vertex_count)?;

        if a == b {
            return Err(TopolyxError::SelfEdge {
                mesh: mesh_index,
                edge: edge_index,
                vertex: a,
            });
        }

        let key = (a.min(b), a.max(b));
        if let Some(&first) = seen.get(&key) {
            return Err(TopolyxError::DuplicateEdge {
                mesh: mesh_index,
                first,
                second: edge_index,
            });
        }
        seen.insert(key, edge_index);
    }

    Ok(())
}

/// Checks that `face_offsets` is a valid partition of the corner range: starts at 0,
/// non-decreasing, and ends at `corners_count` (spec section 4 "face_offsets"). Must run
/// before any code slices `corner_vertices`/`corner_edges` by `face_offsets`.
fn validate_face_offsets_structure(
    mesh_index: usize,
    face_offsets: &[u32],
    corners_count: u32,
) -> Result<(), TopolyxError> {
    let is_valid = match (face_offsets.first(), face_offsets.last()) {
        (Some(&first), Some(&last)) => {
            first == 0 && last == corners_count && face_offsets.windows(2).all(|w| w[0] <= w[1])
        }
        _ => false,
    };

    if !is_valid {
        return Err(TopolyxError::InvalidFaceOffsets { mesh: mesh_index });
    }

    Ok(())
}

/// For each corner `c` of each face and its next corner `n` within the same face (wrapping),
/// checks `corner_edges[c]` connects `corner_vertices[c]` and `corner_vertices[n]`
/// (spec section 5, "Corner and Edge Consistency").
fn validate_corner_edge_consistency(
    mesh_index: usize,
    face_offsets: &[u32],
    corner_vertices: &[u32],
    corner_edges: &[u32],
    edges: &[u32],
) -> Result<(), TopolyxError> {
    for window in face_offsets.windows(2) {
        let (start, end) = (window[0] as usize, window[1] as usize);

        for c in start..end {
            let next = if c + 1 == end { start } else { c + 1 };

            let edge_index = corner_edges[c] as usize;
            let (ev0, ev1) = (edges[edge_index * 2], edges[edge_index * 2 + 1]);
            let (cv0, cv1) = (corner_vertices[c], corner_vertices[next]);

            let connects = (ev0 == cv0 && ev1 == cv1) || (ev0 == cv1 && ev1 == cv0);
            if !connects {
                return Err(TopolyxError::CornerEdgeMismatch {
                    mesh: mesh_index,
                    corner: c as u32,
                });
            }
        }
    }

    Ok(())
}

fn validate_empty_mesh(
    mesh_index: usize,
    counts: &ElementCounts,
    face_offsets: &[u32],
) -> Result<(), TopolyxError> {
    let is_empty = counts.vertices == 0 && counts.edges == 0 && counts.faces == 0 && counts.corners == 0;
    if !is_empty {
        return Ok(());
    }

    // element_counts + validate_element_counts already guarantee face_offsets has exactly one
    // element here; only its *value* still needs checking.
    if face_offsets.first() != Some(&0) {
        return Err(TopolyxError::EmptyMeshFaceOffsets { mesh: mesh_index });
    }

    Ok(())
}

/// Checks that every BIN byte after the last byte referenced by any descriptor in the file
/// is `0x00` (spec section 5, "Container Validity" padding rule, BIN side).
fn validate_bin_tail_padding(meshes: &[Mesh], bin: &[u8]) -> Result<(), TopolyxError> {
    let mut used_end = 0usize;
    for mesh in meshes {
        for descriptor in mesh_descriptors(mesh) {
            let end = descriptor.byte_offset as usize + descriptor.byte_length as usize;
            used_end = used_end.max(end);
        }
    }

    for &byte in &bin[used_end..] {
        if byte != 0 {
            return Err(TopolyxError::InvalidPadding {
                chunk: "BIN",
                expected: 0,
                found: byte,
            });
        }
    }

    Ok(())
}

fn mesh_descriptors(mesh: &Mesh) -> impl Iterator<Item = &DataDescriptor> {
    [
        &mesh.topology.positions,
        &mesh.topology.edges,
        &mesh.topology.corner_vertices,
        &mesh.topology.corner_edges,
        &mesh.topology.face_offsets,
    ]
    .into_iter()
    .chain(mesh.attributes.iter().map(|a| &a.data))
}
