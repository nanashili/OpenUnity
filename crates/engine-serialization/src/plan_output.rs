use crate::{MAX_INPUT_BYTES, Result, SceneError, ScenePlan};
use std::io::{self, Write};

/// Encode a validated scene plan as compact, bounded schema-versioned JSON.
pub fn serialize_plan(plan: &ScenePlan) -> Result<Vec<u8>> {
    plan.validate()?;

    let mut writer = BoundedWriter::new(MAX_INPUT_BYTES);
    serde_json::to_writer(&mut writer, plan).map_err(|error| {
        if writer.exceeded_limit() {
            SceneError::new("serialized plan exceeds output byte limit")
        } else {
            SceneError::new(format!("failed to serialize scene plan: {error}"))
        }
    })?;

    Ok(writer.finish())
}

/// Retain only bytes admitted by the scene transfer payload limit.
struct BoundedWriter {
    bytes: Vec<u8>,
    limit: usize,
    exceeded: bool,
}

impl BoundedWriter {
    fn new(limit: usize) -> Self {
        Self {
            bytes: Vec::new(),
            limit,
            exceeded: false,
        }
    }

    fn exceeded_limit(&self) -> bool {
        self.exceeded
    }

    fn finish(self) -> Vec<u8> {
        self.bytes
    }
}

impl Write for BoundedWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let Some(new_len) = self.bytes.len().checked_add(bytes.len()) else {
            self.exceeded = true;
            return Err(io::Error::new(
                io::ErrorKind::WriteZero,
                "serialized plan output length overflow",
            ));
        };

        if new_len > self.limit {
            self.exceeded = true;
            return Err(io::Error::new(
                io::ErrorKind::WriteZero,
                "serialized plan exceeds output byte limit",
            ));
        }

        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
