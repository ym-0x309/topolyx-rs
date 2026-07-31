use topolyx::error::TopolyxError;
use topolyx::file::{ComponentType, DataDescriptor};
use topolyx::read_topolyx_from_data;

mod common;
use common::{DEFAULT_CUBE_JSON, build_container};

fn descriptor(
    byte_offset: u32,
    byte_length: u32,
    component_type: ComponentType,
    component_count: u32,
    element_count: u32,
) -> DataDescriptor {
    DataDescriptor {
        byte_offset,
        byte_length,
        component_type,
        component_count,
        element_count,
    }
}

#[test]
fn extracts_f32_components() {
    let mut bin = Vec::new();
    for v in [1.0f32, 2.0, 3.0, -4.5, 5.5, 6.0] {
        bin.extend_from_slice(&v.to_le_bytes());
    }
    let d = descriptor(0, 24, ComponentType::F32, 3, 2);

    let data = d.extract(&bin).expect("valid F32 data should decode");

    assert_eq!(data.len(), 6);
    assert_eq!(data.as_f32().unwrap(), &[1.0, 2.0, 3.0, -4.5, 5.5, 6.0]);
    assert!(data.as_u32().is_none());
}

#[test]
fn extracts_i32_components_with_negative_value() {
    let mut bin = Vec::new();
    for v in [10i32, -20, 30] {
        bin.extend_from_slice(&v.to_le_bytes());
    }
    let d = descriptor(0, 12, ComponentType::I32, 1, 3);

    let data = d.extract(&bin).expect("valid I32 data should decode");

    assert_eq!(data.as_i32().unwrap(), &[10, -20, 30]);
}

#[test]
fn extracts_u32_components() {
    let mut bin = Vec::new();
    for v in [0u32, 1, u32::MAX] {
        bin.extend_from_slice(&v.to_le_bytes());
    }
    let d = descriptor(0, 12, ComponentType::U32, 1, 3);

    let data = d.extract(&bin).expect("valid U32 data should decode");

    assert_eq!(data.as_u32().unwrap(), &[0, 1, u32::MAX]);
}

#[test]
fn extracts_i8_components_at_boundary_values() {
    let bin = vec![0x80u8, 0x7F, 0x00]; // -128, 127, 0
    let d = descriptor(0, 3, ComponentType::I8, 1, 3);

    let data = d.extract(&bin).expect("valid I8 data should decode");

    assert_eq!(data.as_i8().unwrap(), &[-128, 127, 0]);
}

#[test]
fn extracts_u8_components_across_full_range() {
    let bin = vec![0u8, 1, 127, 128, 255];
    let d = descriptor(0, 5, ComponentType::U8, 1, 5);

    let data = d.extract(&bin).expect("valid U8 data should decode");

    assert_eq!(data.as_u8().unwrap(), &[0, 1, 127, 128, 255]);
}

#[test]
fn extracts_bool_components_with_valid_bytes() {
    let bin = vec![0x00u8, 0x01, 0x01, 0x00];
    let d = descriptor(0, 4, ComponentType::Bool, 1, 4);

    let data = d.extract(&bin).expect("valid BOOL data should decode");

    assert_eq!(data.as_bool().unwrap(), &[false, true, true, false]);
}

#[test]
fn rejects_invalid_bool_byte_value() {
    let bin = vec![0x00u8, 0x02, 0x01];
    let d = descriptor(0, 3, ComponentType::Bool, 1, 3);

    let err = d.extract(&bin).unwrap_err();

    assert!(matches!(err, TopolyxError::InvalidBoolValue(2)));
}

#[test]
fn rejects_byte_length_mismatch() {
    // component_size(F32)=4 * component_count=3 * element_count=1 = 12, not 11.
    let bin = vec![0u8; 16];
    let d = descriptor(0, 11, ComponentType::F32, 3, 1);

    let err = d.extract(&bin).unwrap_err();

    assert!(matches!(err, TopolyxError::ByteLengthMismatch));
}

#[test]
fn rejects_byte_length_overflow_without_panicking() {
    let bin = vec![0u8; 16];
    let d = descriptor(0, 0, ComponentType::F32, u32::MAX, u32::MAX);

    let err = d.extract(&bin).unwrap_err();

    assert!(matches!(err, TopolyxError::ByteLengthMismatch));
}

#[test]
fn rejects_out_of_bounds_range() {
    let bin = vec![0u8; 8];
    let d = descriptor(0, 12, ComponentType::U32, 1, 3);

    let err = d.extract(&bin).unwrap_err();

    assert!(matches!(err, TopolyxError::DataOutOfBounds));
}

#[test]
fn rejects_out_of_bounds_range_near_u32_max_without_panicking() {
    let bin = vec![0u8; 8];
    // u32::MAX - 3 is 4-byte aligned, keeping this test isolated to the bounds check.
    let d = descriptor(u32::MAX - 3, 8, ComponentType::U8, 1, 8);

    let err = d.extract(&bin).unwrap_err();

    assert!(matches!(err, TopolyxError::DataOutOfBounds));
}

#[test]
fn rejects_unaligned_byte_offset() {
    let bin = vec![0u8; 8];
    let d = descriptor(2, 4, ComponentType::U8, 1, 4);

    let err = d.extract(&bin).unwrap_err();

    assert!(matches!(err, TopolyxError::UnalignedByteOffset(2)));
}

#[test]
fn extracts_empty_array_successfully() {
    let bin: Vec<u8> = Vec::new();
    let d = descriptor(0, 0, ComponentType::U32, 1, 0);

    let data = d.extract(&bin).expect("zero-length data should decode");

    assert!(data.is_empty());
    assert_eq!(data.as_u32().unwrap(), &[] as &[u32]);
}

#[test]
fn extracts_zero_length_array_at_end_of_buffer() {
    let bin = vec![0u8; 4];
    let d = descriptor(4, 0, ComponentType::U32, 1, 0);

    let data = d
        .extract(&bin)
        .expect("byte_offset == bin.len() with zero length should decode");

    assert!(data.is_empty());
}

#[test]
fn end_to_end_decodes_default_cube_fixture() {
    let mut bin = vec![0u8; 804];

    // First position = [1.0, 2.0, 3.0] at byte_offset 0.
    bin[0..4].copy_from_slice(&1.0f32.to_le_bytes());
    bin[4..8].copy_from_slice(&2.0f32.to_le_bytes());
    bin[8..12].copy_from_slice(&3.0f32.to_le_bytes());

    let data = build_container(DEFAULT_CUBE_JSON, &bin);
    let (file, bin) = read_topolyx_from_data(data).expect("valid fixture should parse");

    let mesh = &file.meshes[0];

    let positions = mesh
        .topology
        .positions
        .extract(&bin)
        .expect("positions should decode");
    assert_eq!(positions.len(), 24); // 8 vertices * 3 components
    let positions = positions.as_f32().unwrap();
    assert_eq!(&positions[0..3], &[1.0, 2.0, 3.0]);

    let edges = mesh
        .topology
        .edges
        .extract(&bin)
        .expect("edges should decode");
    assert_eq!(edges.len(), 24); // 12 edges * 2 components

    let sharp_face = mesh.attributes[0]
        .data
        .extract(&bin)
        .expect("sharp_face (BOOL) should decode");
    assert_eq!(sharp_face.as_bool().unwrap(), &[false; 6]);

    let uv_map = mesh.attributes[1]
        .data
        .extract(&bin)
        .expect("UVMap (F32x2) should decode");
    assert_eq!(uv_map.len(), 48); // 24 corners * 2 components

    let custom_attribute = mesh.attributes[2]
        .data
        .extract(&bin)
        .expect("custom_attribute (F32x4) should decode");
    assert_eq!(custom_attribute.len(), 48); // 12 edges * 4 components
}
