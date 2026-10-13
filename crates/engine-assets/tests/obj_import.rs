//! Public triangle OBJ grammar, bounds, and typed render-asset acceptance.

use engine_assets::{
    AssetError, MeshAsset, ObjImportError, RenderAssets, UnlitMaterial, parse_obj_mesh,
};
use std::error::Error;

const INPUT_LIMIT: usize = 16 * 1024 * 1024;
const POSITION_LIMIT: usize = 65_536;
const INDEX_LIMIT: usize = 196_608;

#[test]
fn triangle_mapping_preserves_ascii_order_and_caller_id() {
    let source = b"# triangle\n  v 10.5 -2 3\r\n\tv 0 4 5\r\nv 8 9 1\n\nf 3 1 2\n";

    let mesh = parse_obj_mesh(41, source).expect("valid triangle should parse");

    assert_eq!(mesh.id(), 41);
    assert_eq!(
        mesh.positions(),
        &[[10.5, -2.0, 3.0], [0.0, 4.0, 5.0], [8.0, 9.0, 1.0]]
    );
    assert_eq!(mesh.indices(), &[2, 0, 1]);
}

#[test]
fn parser_accepts_comments_blank_lines_and_final_unterminated_record() {
    let source = b"\n# leading comment\n\tv 0 0 0\n\n v 1 0 0\n v 0 1 0\n# face follows\nf 1 2 3";

    let mesh = parse_obj_mesh(1, source).expect("valid line records should parse");

    assert_eq!(mesh.positions().len(), 3);
    assert_eq!(mesh.indices(), &[0, 1, 2]);
}

#[test]
fn parser_accepts_decimal_forms_and_finite_underflow() {
    let source = b"v 1e1 .5 2.\nv -1e-999 +3.0 4E-2\nv 0 1 0\nv 1e-45 -0 0\nf 1 2 3\n";

    let mesh = parse_obj_mesh(2, source).expect("valid decimal grammar should parse");

    assert_eq!(mesh.positions()[0], [10.0, 0.5, 2.0]);
    assert!(mesh.positions()[1][0].is_sign_negative());
    assert_eq!(mesh.positions()[1][1], 3.0);
    assert_eq!(mesh.positions()[1][2], 0.04);
    assert_eq!(mesh.positions()[3][0].to_bits(), 1);
    assert!(mesh.positions()[3][1].is_sign_negative());
}

#[test]
fn malformed_and_unsupported_records_fail_closed() {
    let cases = [
        (b"v 0 0\n".as_slice(), "vertex arity"),
        (b"v nope 0 0\n".as_slice(), "malformed coordinate"),
        (b"v NaN 0 0\n".as_slice(), "non-finite coordinate"),
        (b"v 1e999 0 0\n".as_slice(), "coordinate overflow"),
        (
            b"v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2\n".as_slice(),
            "short face",
        ),
        (
            b"v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3 1\n".as_slice(),
            "long face",
        ),
        (
            b"v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1/1 2/2 3/3\n".as_slice(),
            "slash face",
        ),
        (
            b"v 0 0 0\nv 1 0 0\nv 0 1 0\nf -3 2 3\n".as_slice(),
            "negative face",
        ),
        (b"vt 0 0\n".as_slice(), "texture statement"),
        (b"v 0 0 0\rv 1 0 0\n".as_slice(), "invalid carriage return"),
        (
            b"v 0 0 0\nv 1 0 0\nv 0 1 0\n# inline\nf 1 2 3 # nope\n".as_slice(),
            "inline comment",
        ),
    ];

    for (source, description) in cases {
        assert!(
            parse_obj_mesh(1, source).is_err(),
            "{description} must fail"
        );
    }
}

#[test]
fn parser_reports_stable_categories_and_locations() {
    assert!(matches!(
        parse_obj_mesh(1, b"x 0 0 0\n"),
        Err(ObjImportError::UnsupportedStatement { line: 1 })
    ));
    assert!(matches!(
        parse_obj_mesh(1, b"v 0 0 0\nv 1 0 0\nv 0 1 0\nf 0 1 2\n"),
        Err(ObjImportError::InvalidFaceReference { line: 4 })
    ));
    assert!(matches!(
        parse_obj_mesh(1, b"v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 4\n"),
        Err(ObjImportError::InvalidFaceReference { line: 4 })
    ));
    assert!(matches!(
        parse_obj_mesh(1, b"v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1/1 2/2 3/3\n"),
        Err(ObjImportError::UnsupportedFaceReference { line: 4 })
    ));
    assert!(matches!(
        parse_obj_mesh(1, b"v 0 0 0\nv 1 0 0\nv 0 1 0\nf -3 2 3\n"),
        Err(ObjImportError::UnsupportedFaceReference { line: 4 })
    ));
    assert!(matches!(
        parse_obj_mesh(
            1,
            b"v 0 0 0\nv 1 0 0\nv 0 1 0\nf 999999999999999999999999 1 2\n"
        ),
        Err(ObjImportError::InvalidFaceReference { line: 4 })
    ));
    assert!(matches!(
        parse_obj_mesh(1, b"\xff"),
        Err(ObjImportError::NonAscii { byte_offset: 0 })
    ));
    assert!(matches!(
        parse_obj_mesh(1, b"\r"),
        Err(ObjImportError::InvalidTextByte { byte_offset: 0 })
    ));
    assert!(matches!(
        parse_obj_mesh(1, b"# \xff"),
        Err(ObjImportError::NonAscii { byte_offset: 2 })
    ));
    assert!(matches!(
        parse_obj_mesh(1, b"# \0"),
        Err(ObjImportError::InvalidTextByte { byte_offset: 2 })
    ));
    assert!(matches!(
        parse_obj_mesh(1, b"# \x01"),
        Err(ObjImportError::InvalidTextByte { byte_offset: 2 })
    ));
    assert!(matches!(
        parse_obj_mesh(1, b"v 0 0\n"),
        Err(ObjImportError::MalformedVertex { line: 1 })
    ));
    assert!(matches!(
        parse_obj_mesh(1, b"v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2\n"),
        Err(ObjImportError::UnsupportedFaceArity { line: 4 })
    ));
}

#[test]
fn every_named_unsupported_record_fails_with_a_stable_category() {
    let statements = [
        b"vt 0 0".as_slice(),
        b"vn 0 0 1".as_slice(),
        b"vp 0 0 0".as_slice(),
        b"p 1".as_slice(),
        b"l 1 2".as_slice(),
        b"o object".as_slice(),
        b"g group".as_slice(),
        b"usemtl material".as_slice(),
        b"mtllib scene.mtl".as_slice(),
        b"s 1".as_slice(),
        b"curv 0 1 1 2".as_slice(),
    ];

    for statement in statements {
        assert!(matches!(
            parse_obj_mesh(1, statement),
            Err(ObjImportError::UnsupportedStatement { line: 1 })
        ));
    }

    assert!(parse_obj_mesh(1, b"v 0 0 0 \\\n").is_err());
}

#[test]
fn parser_rejects_atomic_geometry_failures() {
    assert!(matches!(
        parse_obj_mesh(0, valid_triangle()),
        Err(ObjImportError::Mesh(AssetError::Limit("mesh ID")))
    ));
    assert!(matches!(
        parse_obj_mesh(1, b"v 0 0 0\nv 1 0 0\nv 0 1 0\n"),
        Err(ObjImportError::Mesh(AssetError::Limit("mesh indices")))
    ));
    assert!(matches!(
        parse_obj_mesh(1, b"v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\nf 4 2 3\n"),
        Err(ObjImportError::InvalidFaceReference { line: 5 })
    ));
    assert!(matches!(
        parse_obj_mesh(1, b"f 1 2 3\nv 0 0 0\nv 1 0 0\nv 0 1 0\n"),
        Err(ObjImportError::InvalidFaceReference { line: 1 })
    ));
}

#[test]
fn importer_error_exposes_wrapped_mesh_error() {
    let error = parse_obj_mesh(0, valid_triangle()).unwrap_err();

    assert!(error.to_string().contains("mesh ID"));
    assert!(error.source().is_some());
}

#[test]
fn input_limit_accepts_exact_boundary_and_rejects_one_byte_over() {
    let exact = padded_valid_obj(INPUT_LIMIT);
    let mesh = parse_obj_mesh(7, &exact).expect("exact input bound should be accepted");
    assert_eq!(mesh.indices(), &[0, 1, 2]);

    let oversized = padded_valid_obj(INPUT_LIMIT + 1);
    assert!(matches!(
        parse_obj_mesh(7, &oversized),
        Err(ObjImportError::InputTooLarge {
            byte_offset: INPUT_LIMIT
        })
    ));
}

#[test]
fn position_limit_accepts_exact_boundary_and_rejects_one_over() {
    let exact = obj_with_vertices(POSITION_LIMIT);
    let mesh = parse_obj_mesh(9, &exact).expect("exact position bound should be accepted");
    assert_eq!(mesh.positions().len(), POSITION_LIMIT);

    let oversized = obj_with_vertices(POSITION_LIMIT + 1);
    assert!(matches!(
        parse_obj_mesh(9, &oversized),
        Err(ObjImportError::PositionLimitExceeded { .. })
    ));
}

#[test]
fn index_limit_accepts_exact_boundary_and_rejects_one_over() {
    let exact = obj_with_faces(INDEX_LIMIT / 3);
    let mesh = parse_obj_mesh(10, &exact).expect("exact index bound should be accepted");
    assert_eq!(mesh.indices().len(), INDEX_LIMIT);

    let oversized = obj_with_faces(INDEX_LIMIT / 3 + 1);
    assert!(matches!(
        parse_obj_mesh(10, &oversized),
        Err(ObjImportError::IndexLimitExceeded { .. })
    ));
}

#[test]
fn typed_assembly_preserves_order_and_allows_cross_kind_id_reuse() {
    let first = mesh(11, 0.0);
    let second = mesh(12, 1.0);
    let material = UnlitMaterial::try_new(11, [10, 20, 30, 255]).unwrap();

    let assets = RenderAssets::try_new(vec![first, second], vec![material]).unwrap();

    assert_eq!(assets.schema_version(), 1);
    assert_eq!(
        assets
            .meshes()
            .iter()
            .map(MeshAsset::id)
            .collect::<Vec<_>>(),
        [11, 12]
    );
    assert_eq!(assets.meshes()[0].positions()[0], [0.0, 0.0, 0.0]);
    assert_eq!(assets.meshes()[1].positions()[0], [1.0, 0.0, 0.0]);
    assert_eq!(assets.materials()[0].id(), 11);
}

#[test]
fn typed_assembly_rejects_same_kind_duplicate_ids() {
    let result = RenderAssets::try_new(vec![mesh(4, 0.0), mesh(4, 1.0)], Vec::new());

    assert!(matches!(result, Err(AssetError::DuplicateId("mesh"))));

    let materials = vec![
        UnlitMaterial::try_new(5, [0, 0, 0, 255]).unwrap(),
        UnlitMaterial::try_new(5, [1, 1, 1, 255]).unwrap(),
    ];
    assert!(matches!(
        RenderAssets::try_new(Vec::new(), materials),
        Err(AssetError::DuplicateId("material"))
    ));
}

#[test]
fn typed_assembly_enforces_mesh_count_and_aggregate_limits() {
    let too_many_meshes = (1..=257).map(|id| mesh(id, id as f32)).collect();
    assert!(matches!(
        RenderAssets::try_new(too_many_meshes, Vec::new()),
        Err(AssetError::Limit("render asset count"))
    ));

    let position_overflow = vec![
        large_mesh(1, POSITION_LIMIT, 3),
        large_mesh(2, POSITION_LIMIT, 3),
        large_mesh(3, POSITION_LIMIT, 3),
        large_mesh(4, POSITION_LIMIT, 3),
        mesh(5, 5.0),
    ];
    assert!(matches!(
        RenderAssets::try_new(position_overflow, Vec::new()),
        Err(AssetError::Limit("mesh positions"))
    ));

    let index_overflow = vec![
        large_mesh(1, 3, INDEX_LIMIT),
        large_mesh(2, 3, INDEX_LIMIT),
        large_mesh(3, 3, INDEX_LIMIT),
        large_mesh(4, 3, INDEX_LIMIT),
        mesh(5, 5.0),
    ];
    assert!(matches!(
        RenderAssets::try_new(index_overflow, Vec::new()),
        Err(AssetError::Limit("mesh indices"))
    ));
}

#[test]
fn typed_assembly_accepts_material_limit_and_rejects_one_over() {
    let exact = (1..=256)
        .map(|id| UnlitMaterial::try_new(id, [0, 0, 0, 255]).unwrap())
        .collect();
    let assets = RenderAssets::try_new(Vec::new(), exact).unwrap();
    assert_eq!(assets.materials().len(), 256);

    let too_many = (1..=257)
        .map(|id| UnlitMaterial::try_new(id, [0, 0, 0, 255]).unwrap())
        .collect();
    assert!(matches!(
        RenderAssets::try_new(Vec::new(), too_many),
        Err(AssetError::Limit("render asset count"))
    ));
}

#[test]
fn json_path_retains_existing_schema_and_input_bounds() {
    let empty = RenderAssets::parse_json(br#"{"schema_version":1,"meshes":[],"materials":[]}"#)
        .expect("existing JSON path should remain available");
    assert_eq!(empty.schema_version(), 1);

    let oversized = vec![b' '; INPUT_LIMIT + 1];
    assert!(matches!(
        RenderAssets::parse_json(&oversized),
        Err(AssetError::Limit("JSON input"))
    ));
}

fn valid_triangle() -> &'static [u8] {
    b"v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n"
}

fn padded_valid_obj(length: usize) -> Vec<u8> {
    let mut bytes = valid_triangle().to_vec();
    bytes.resize(length, b'#');
    bytes
}

fn obj_with_vertices(count: usize) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(count * 16 + 16);
    for index in 0..count {
        bytes.extend_from_slice(format!("v {index} 0 0\n").as_bytes());
    }
    bytes.extend_from_slice(format!("f 1 2 {count}\n").as_bytes());
    bytes
}

fn obj_with_faces(face_count: usize) -> Vec<u8> {
    let mut bytes = b"v 0 0 0\nv 1 0 0\nv 0 1 0\n".to_vec();
    bytes.reserve(face_count * 8);
    for _ in 0..face_count {
        bytes.extend_from_slice(b"f 1 2 3\n");
    }
    bytes
}

fn mesh(id: u32, offset: f32) -> MeshAsset {
    MeshAsset::try_new(
        id,
        vec![
            [offset, 0.0, 0.0],
            [offset + 1.0, 0.0, 0.0],
            [offset, 1.0, 0.0],
        ],
        vec![0, 1, 2],
    )
    .unwrap()
}

fn large_mesh(id: u32, position_count: usize, index_count: usize) -> MeshAsset {
    let positions = (0..position_count)
        .map(|index| [index as f32, 0.0, 0.0])
        .collect();
    let indices = (0..index_count).map(|index| (index % 3) as u32).collect();

    MeshAsset::try_new(id, positions, indices).unwrap()
}
