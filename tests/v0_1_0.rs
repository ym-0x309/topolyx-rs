use topolyx::read_topolyx_from_data;

mod common;
use common::{DEFAULT_CUBE_JSON, build_container};

#[test]
fn read_topolyx_parses_default_cube_fixture() {
    // custom_attribute가 612..804 바이트를 차지하므로 BIN은 최소 804바이트 필요
    let data = build_container(DEFAULT_CUBE_JSON, &vec![0u8; 804]);

    let result = read_topolyx_from_data(&data);

    let (file, bin) = result.expect("valid fixture should parse without error");

    assert_eq!(file.header.format, "Topolyx");
    assert_eq!(file.coordinate_system.up_axis, "+Z");
    assert_eq!(file.objects.len(), 1);
    assert_eq!(file.meshes.len(), 1);

    let mesh = &file.meshes[0];
    assert_eq!(mesh.element_counts.vertices, 8);
    assert_eq!(mesh.attributes.len(), 3);
    assert_eq!(mesh.topology.positions.element_count, 8);

    assert_eq!(bin.len(), 804);
}
