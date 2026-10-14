//! Scale-aware Transform vector conversion without translation.

use crate::{Runtime, RuntimeError, math};
use engine_object::ObjectId;

impl Runtime {
    /// Convert a vector through the complete ancestor chain without translation.
    ///
    /// The forward conversion applies each local scale and rotation. The inverse
    /// conversion reverses that sequence and can fail for a singular scale.
    /// # Errors
    /// Returns object/kind, non-finite numeric, or singular inverse errors.
    pub fn transform_vector(
        &self,
        id: ObjectId,
        mut vector: math::Vector,
        inverse: bool,
    ) -> Result<math::Vector, RuntimeError> {
        self.transform(id)?;
        math::finite(&vector)?;

        if inverse {
            vector = self.inverse_transform_vector(id, vector)?;
        } else {
            vector = self.forward_transform_vector(id, vector)?;
        }

        math::finite(&vector)?;
        Ok(vector)
    }

    fn forward_transform_vector(
        &self,
        id: ObjectId,
        mut vector: math::Vector,
    ) -> Result<math::Vector, RuntimeError> {
        let mut current = Some(id);
        while let Some(current_id) = current {
            let data = self.transform(current_id)?;
            vector = math::rotate(data.rotation, math::scale(vector, data.scale)?)?;
            current = self.transform_parent_id(current_id)?;
        }

        Ok(vector)
    }

    fn inverse_transform_vector(
        &self,
        id: ObjectId,
        mut vector: math::Vector,
    ) -> Result<math::Vector, RuntimeError> {
        let mut chain = Vec::new();
        let mut current = Some(id);
        while let Some(current_id) = current {
            chain.push(current_id);
            current = self.transform_parent_id(current_id)?;
        }

        for current_id in chain.into_iter().rev() {
            let data = self.transform(current_id)?;
            vector = math::divide(
                math::rotate(math::conjugate(data.rotation), vector)?,
                data.scale,
            )?;
        }

        Ok(vector)
    }
}
