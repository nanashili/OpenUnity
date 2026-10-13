//! Checks asset-object catalog behavior through the public `engine-assets` API.

use engine_assets::{
    AssetObjectBinding, AssetObjectCatalog, AssetObjectCatalogBuildError, AssetObjectClass,
    AssetObjectLookupError, AssetObjectReference, AssetObjectTarget, Guid, InvalidReference,
    MeshAsset, Metadata, MetadataIndex, RenderAssets, UnlitMaterial, parse_obj_mesh,
};
use std::cell::Cell;
use std::rc::Rc;

const GUID_A_TEXT: &str = "11111111111111111111111111111111";
const GUID_B_TEXT: &str = "22222222222222222222222222222222";
const ZERO_GUID_TEXT: &str = "00000000000000000000000000000000";
const SHARED_LOCAL_ID: i64 = 5_000_000_101;
const SECOND_LOCAL_ID: i64 = 5_000_000_202;
const LARGE_LOCAL_ID: i64 = u32::MAX as i64 + 2;

fn guid(text: &str) -> Guid {
    text.parse().expect("fixture GUID is valid hexadecimal")
}

fn reference(guid: Guid, local_file_id: i64) -> AssetObjectReference {
    AssetObjectReference::new(guid, local_file_id)
        .expect("fixture reference should be admitted by the key contract")
}

fn binding(reference: AssetObjectReference, target: AssetObjectTarget) -> AssetObjectBinding {
    AssetObjectBinding::new(reference, target)
}

fn metadata_for(guids: &[Guid]) -> MetadataIndex {
    let mut metadata = MetadataIndex::new();
    for (index, guid) in guids.iter().copied().enumerate() {
        let path = format!("virtual/asset-{index}.asset");
        metadata
            .insert(path, Metadata { guid })
            .expect("fixture GUIDs have one owner each");
    }
    metadata
}

fn render_assets_with_obj_fixture() -> RenderAssets {
    let imported_mesh = parse_obj_mesh(17, include_bytes!("fixtures/catalog_triangle.obj"))
        .expect("the checked-in triangle is valid OBJ input");
    let second_mesh = mesh(18, [[10.0, 0.0, 0.0], [11.0, 0.0, 0.0], [10.0, 1.0, 0.0]]);
    let material = UnlitMaterial::try_new(4, [12, 34, 56, 255]).expect("fixture material is valid");

    RenderAssets::try_new(vec![imported_mesh, second_mesh], vec![material])
        .expect("fixture resources have valid IDs and payloads")
}

fn catalog_with_one_mesh(
    guid: Guid,
    reference: AssetObjectReference,
    mesh: MeshAsset,
) -> AssetObjectCatalog {
    let target = AssetObjectTarget::Mesh(mesh.id());
    let assets = RenderAssets::try_new(vec![mesh], Vec::new()).unwrap();
    let bindings = vec![binding(reference, target)];

    AssetObjectCatalog::try_new(metadata_for(&[guid]), assets, bindings).unwrap()
}

fn mesh(id: u32, positions: [[f32; 3]; 3]) -> MeshAsset {
    MeshAsset::try_new(id, positions.to_vec(), vec![0, 1, 2]).unwrap()
}

fn assert_mesh(mesh: &MeshAsset, id: u32, positions: &[[f32; 3]], indices: &[u32]) {
    assert_eq!(mesh.id(), id);
    assert_eq!(mesh.positions(), positions);
    assert_eq!(mesh.indices(), indices);
}

fn assert_mesh_resolution(
    catalog: &AssetObjectCatalog,
    reference: &AssetObjectReference,
    id: u32,
    positions: &[[f32; 3]],
    indices: &[u32],
) {
    let resolved_mesh = catalog
        .resolve_mesh(reference)
        .expect("fixture reference should resolve to a mesh");
    assert_mesh(resolved_mesh, id, positions, indices);
}

struct CountedBindings {
    guid: Guid,
    next_local_file_id: i64,
    remaining: usize,
    yielded_count: Rc<Cell<usize>>,
}

impl CountedBindings {
    fn new(guid: Guid, count: usize, yielded_count: Rc<Cell<usize>>) -> Self {
        Self {
            guid,
            next_local_file_id: 1,
            remaining: count,
            yielded_count,
        }
    }
}

impl Iterator for CountedBindings {
    type Item = AssetObjectBinding;

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            return None;
        }

        let local_file_id = self.next_local_file_id;
        self.next_local_file_id += 1;
        self.remaining -= 1;
        self.yielded_count.set(self.yielded_count.get() + 1);

        Some(binding(
            reference(self.guid, local_file_id),
            AssetObjectTarget::Mesh(17),
        ))
    }
}

#[test]
fn resolves_the_full_signed_composite_key_to_typed_payloads() {
    let guid_a = guid(GUID_A_TEXT);
    let guid_b = guid(GUID_B_TEXT);
    let shared = reference(guid_a, SHARED_LOCAL_ID);
    let repeated_shared = reference(guid_a, SHARED_LOCAL_ID);
    let second_object = reference(guid_a, SECOND_LOCAL_ID);
    let same_local_id_other_guid = reference(guid_b, SHARED_LOCAL_ID);
    let negative_local_id = reference(guid_a, -71);
    let large_local_id = reference(guid_a, LARGE_LOCAL_ID);
    let material_reference = reference(guid_a, 303);

    let bindings = vec![
        binding(shared, AssetObjectTarget::Mesh(17)),
        binding(second_object, AssetObjectTarget::Mesh(18)),
        binding(same_local_id_other_guid, AssetObjectTarget::Mesh(18)),
        binding(negative_local_id, AssetObjectTarget::Mesh(17)),
        binding(large_local_id, AssetObjectTarget::Mesh(18)),
        binding(material_reference, AssetObjectTarget::UnlitMaterial(4)),
    ];
    let catalog = AssetObjectCatalog::try_new(
        metadata_for(&[guid_a, guid_b]),
        render_assets_with_obj_fixture(),
        bindings,
    )
    .expect("valid explicit bindings should build a catalog");

    let obj_positions = [[1.0, 2.0, 3.0], [4.0, 2.0, 3.0], [1.0, 5.0, 3.0]];
    let alternate_positions = [[10.0, 0.0, 0.0], [11.0, 0.0, 0.0], [10.0, 1.0, 0.0]];
    assert_mesh_resolution(&catalog, &shared, 17, &obj_positions, &[0, 1, 2]);
    assert_mesh_resolution(&catalog, &repeated_shared, 17, &obj_positions, &[0, 1, 2]);
    assert_mesh_resolution(
        &catalog,
        &second_object,
        18,
        &alternate_positions,
        &[0, 1, 2],
    );
    assert_mesh_resolution(
        &catalog,
        &same_local_id_other_guid,
        18,
        &alternate_positions,
        &[0, 1, 2],
    );
    assert_mesh_resolution(&catalog, &negative_local_id, 17, &obj_positions, &[0, 1, 2]);
    assert_mesh_resolution(
        &catalog,
        &large_local_id,
        18,
        &alternate_positions,
        &[0, 1, 2],
    );

    let material = catalog.resolve_material(&material_reference).unwrap();
    assert_eq!(material.id(), 4);
    assert_eq!(material.rgba8(), [12, 34, 56, 255]);
}

#[test]
fn same_key_in_separate_catalogs_resolves_each_catalogs_own_payload() {
    let guid_a = guid(GUID_A_TEXT);
    let shared_reference = reference(guid_a, 8_001);
    let first_catalog = catalog_with_one_mesh(
        guid_a,
        shared_reference,
        mesh(31, [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]]),
    );
    let second_catalog = catalog_with_one_mesh(
        guid_a,
        shared_reference,
        mesh(32, [[20.0, 0.0, 0.0], [22.0, 0.0, 0.0], [20.0, 2.0, 0.0]]),
    );

    assert_mesh(
        first_catalog.resolve_mesh(&shared_reference).unwrap(),
        31,
        &[[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
        &[0, 1, 2],
    );
    assert_mesh(
        second_catalog.resolve_mesh(&shared_reference).unwrap(),
        32,
        &[[20.0, 0.0, 0.0], [22.0, 0.0, 0.0], [20.0, 2.0, 0.0]],
        &[0, 1, 2],
    );
}

#[test]
fn lookup_errors_distinguish_unindexed_guid_missing_local_id_and_wrong_class() {
    let guid_a = guid(GUID_A_TEXT);
    let guid_b = guid(GUID_B_TEXT);
    let missing_guid = reference(guid_b, SHARED_LOCAL_ID);
    let missing_local_id = reference(guid_a, 404);
    let material_reference = reference(guid_a, 405);
    let catalog = AssetObjectCatalog::try_new(
        metadata_for(&[guid_a]),
        render_assets_with_obj_fixture(),
        vec![binding(
            material_reference,
            AssetObjectTarget::UnlitMaterial(4),
        )],
    )
    .unwrap();

    let error = catalog.resolve_mesh(&missing_guid).unwrap_err();
    assert!(matches!(
        error,
        AssetObjectLookupError::MissingAssetGuid { reference: found }
            if found == missing_guid
    ));

    let error = catalog.resolve_mesh(&missing_local_id).unwrap_err();
    assert!(matches!(
        error,
        AssetObjectLookupError::MissingLocalFileId { reference: found }
            if found == missing_local_id
    ));

    let error = catalog.resolve_mesh(&material_reference).unwrap_err();
    match error {
        AssetObjectLookupError::WrongAssetClass {
            reference: found,
            expected,
            actual,
        } => {
            assert_eq!(found, material_reference);
            assert_eq!(expected, AssetObjectClass::Mesh);
            assert_eq!(actual, AssetObjectClass::UnlitMaterial);
        }
        other => panic!("unexpected lookup error: {other:?}"),
    }
}

#[test]
fn build_errors_reject_unindexed_and_duplicate_references() {
    let guid_a = guid(GUID_A_TEXT);
    let guid_b = guid(GUID_B_TEXT);
    let unindexed_reference = reference(guid_b, 17);
    let unindexed_error = AssetObjectCatalog::try_new(
        metadata_for(&[guid_a]),
        render_assets_with_obj_fixture(),
        vec![AssetObjectBinding::new(
            unindexed_reference,
            AssetObjectTarget::Mesh(17),
        )],
    )
    .unwrap_err();
    assert!(matches!(
        unindexed_error,
        AssetObjectCatalogBuildError::MissingAssetGuid { reference: found }
            if found == unindexed_reference
    ));

    let duplicate_reference = reference(guid_a, 18);
    let duplicate_error = AssetObjectCatalog::try_new(
        metadata_for(&[guid_a]),
        render_assets_with_obj_fixture(),
        vec![
            AssetObjectBinding::new(duplicate_reference, AssetObjectTarget::Mesh(17)),
            AssetObjectBinding::new(duplicate_reference, AssetObjectTarget::UnlitMaterial(4)),
        ],
    )
    .unwrap_err();
    assert!(matches!(
        duplicate_error,
        AssetObjectCatalogBuildError::DuplicateReference { reference: found }
            if found == duplicate_reference
    ));
}

#[test]
fn build_errors_retain_missing_target_details_and_stop_at_binding_limit() {
    let guid_a = guid(GUID_A_TEXT);
    let missing_reference = reference(guid_a, 1);
    let missing_target = AssetObjectTarget::Mesh(999);
    let missing_target_error = AssetObjectCatalog::try_new(
        metadata_for(&[guid_a]),
        render_assets_with_obj_fixture(),
        vec![AssetObjectBinding::new(missing_reference, missing_target)],
    )
    .unwrap_err();
    assert!(matches!(
        missing_target_error,
        AssetObjectCatalogBuildError::MissingRenderTarget {
            reference: found_reference,
            target: found_target,
        } if found_reference == missing_reference && found_target == missing_target
    ));

    let yielded_count = Rc::new(Cell::new(0));
    let too_many_bindings = CountedBindings::new(guid_a, 300, Rc::clone(&yielded_count));
    let limit_error = expect_build_error(AssetObjectCatalog::try_new(
        metadata_for(&[guid_a]),
        render_assets_with_obj_fixture(),
        too_many_bindings,
    ));
    assert!(matches!(
        limit_error,
        AssetObjectCatalogBuildError::BindingLimitExceeded { max_bindings: 256 }
    ));
    assert_eq!(yielded_count.get(), 257);
}

#[test]
fn catalog_accepts_exactly_256_bindings() {
    let guid_a = guid(GUID_A_TEXT);
    let yielded_count = Rc::new(Cell::new(0));
    let bindings = CountedBindings::new(guid_a, 256, Rc::clone(&yielded_count));
    let catalog = AssetObjectCatalog::try_new(
        metadata_for(&[guid_a]),
        render_assets_with_obj_fixture(),
        bindings,
    )
    .expect("the documented maximum binding count should be accepted");

    assert_eq!(yielded_count.get(), 256);
    assert_eq!(
        catalog.resolve_mesh(&reference(guid_a, 256)).unwrap().id(),
        17
    );
}

#[test]
fn metadata_index_keeps_duplicate_guid_ownership_as_its_own_error() {
    let guid_a = guid(GUID_A_TEXT);
    let mut metadata = MetadataIndex::new();
    metadata
        .insert("virtual/first.asset", Metadata { guid: guid_a })
        .unwrap();

    let error = metadata
        .insert("virtual/second.asset", Metadata { guid: guid_a })
        .unwrap_err();

    assert_eq!(error.guid, guid_a);
    assert_eq!(
        error.first_path,
        std::path::PathBuf::from("virtual/first.asset")
    );
    assert_eq!(
        error.duplicate_path,
        std::path::PathBuf::from("virtual/second.asset")
    );
}

fn expect_build_error(
    result: Result<AssetObjectCatalog, AssetObjectCatalogBuildError>,
) -> AssetObjectCatalogBuildError {
    match result {
        Err(error) => error,
        Ok(_) => panic!("fixture catalog unexpectedly built successfully"),
    }
}

#[test]
fn reference_constructor_rejects_zero_local_id_and_all_zero_guid() {
    let guid_a = guid(GUID_A_TEXT);
    let zero_local_id = AssetObjectReference::new(guid_a, 0).unwrap_err();
    assert!(matches!(
        zero_local_id,
        InvalidReference::ZeroLocalFileId(0)
    ));

    let zero_guid = guid(ZERO_GUID_TEXT);
    let all_zero_guid = AssetObjectReference::new(zero_guid, 9).unwrap_err();
    assert!(matches!(
        all_zero_guid,
        InvalidReference::AllZeroGuid(found) if found == zero_guid
    ));
}
