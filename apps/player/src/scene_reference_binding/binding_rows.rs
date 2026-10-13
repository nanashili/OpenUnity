use super::types::{AssetBindingRow, BindingKind, PointerResolutionError};
use engine_assets::{AssetObjectBinding, AssetObjectReference, AssetObjectTarget, Guid};
use std::collections::BTreeSet;
use std::str::FromStr;

pub(super) struct ConvertedBindings {
    pub(super) catalog_rows: Vec<AssetObjectBinding>,
    pub(super) authored_references: Vec<AssetObjectReference>,
    pub(super) unused: BTreeSet<AssetObjectReference>,
}

pub(super) fn convert_rows(
    rows: &[AssetBindingRow],
) -> Result<ConvertedBindings, PointerResolutionError> {
    let mut references = BTreeSet::new();
    let mut authored_references = Vec::with_capacity(rows.len());
    let mut converted = Vec::with_capacity(rows.len());
    for (index, row) in rows.iter().enumerate() {
        let guid =
            Guid::from_str(&row.guid).map_err(|error| binding_failure(index, error.to_string()))?;
        let reference = AssetObjectReference::new(guid, row.local_file_id)
            .map_err(|error| binding_failure(index, error.to_string()))?;
        if !references.insert(reference) {
            return Err(binding_failure(
                index,
                format!(
                    "duplicate asset-object binding {guid}:{}",
                    row.local_file_id
                ),
            ));
        }
        authored_references.push(reference);
        let target = match row.kind {
            BindingKind::Mesh => AssetObjectTarget::Mesh(row.resource_id),
            BindingKind::UnlitMaterial => AssetObjectTarget::UnlitMaterial(row.resource_id),
        };
        converted.push(AssetObjectBinding::new(reference, target));
    }
    Ok(ConvertedBindings {
        catalog_rows: converted,
        authored_references,
        unused: references,
    })
}

fn binding_failure(index: usize, detail: String) -> PointerResolutionError {
    PointerResolutionError::new(format!("render-asset-reference: binding {index}: {detail}"))
}
