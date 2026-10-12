/// Validated unlit color material.
#[derive(Clone, Debug, Eq, PartialEq)]
/// Validated opaque unlit material data.
pub struct UnlitMaterial {
    id: u32,
    rgba8: [u8; 4],
}

impl UnlitMaterial {
    /// Creates a material with exactly four color bytes and opaque alpha.
    pub fn try_new(id: u32, rgba8: [u8; 4]) -> Result<Self, super::mesh_asset::AssetError> {
        if id == 0 || id > u16::MAX as u32 {
            return Err(super::mesh_asset::AssetError::Limit("material ID"));
        }
        if rgba8[3] != 255 {
            return Err(super::mesh_asset::AssetError::Numeric("material alpha"));
        }
        Ok(Self { id, rgba8 })
    }

    /// Returns the stable material identifier.
    pub fn id(&self) -> u32 {
        self.id
    }

    /// Returns the material color in RGBA byte order.
    pub fn rgba8(&self) -> [u8; 4] {
        self.rgba8
    }
}
