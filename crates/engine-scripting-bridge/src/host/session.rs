use super::ManagedHost;
use super::snapshot::{SceneSnapshot, decode, required_size};
use crate::HostError;
use std::io::Write;

/// An owned managed script session, borrowing its native host.
pub struct ScriptSession<'a> {
    host: &'a ManagedHost,
    id: Option<i64>,
}

impl<'a> ScriptSession<'a> {
    pub(super) fn new(host: &'a ManagedHost, id: i64) -> Self {
        Self { host, id: Some(id) }
    }

    fn invoke(&mut self, opcode: i32, value: i64) -> Result<i64, HostError> {
        let mut output = 0_i64;
        // SAFETY: owned live session and aligned output storage valid for the call.
        let status = unsafe {
            (self.host.invoke)(self.id.expect("owned session"), opcode, value, &mut output)
        };
        if status == 0 {
            Ok(output)
        } else {
            Err(HostError::status("managed callback", status))
        }
    }

    /// Initialize this session exactly once.
    pub fn initialize(&mut self, value: i64) -> Result<i64, HostError> {
        self.invoke(1, value)
    }
    /// Invoke one tick after initialization and before shutdown.
    pub fn tick(&mut self, value: i64) -> Result<i64, HostError> {
        self.invoke(2, value)
    }
    /// Invoke explicit script shutdown; this does not destroy the session.
    pub fn shutdown(&mut self, value: i64) -> Result<i64, HostError> {
        self.invoke(3, value)
    }
    /// Release this script's managed references without invoking a callback.
    pub fn close(mut self) -> Result<(), HostError> {
        // SAFETY: self owns the handle; only successful release clears its ownership.
        let status = unsafe { (self.host.destroy)(self.id.expect("owned session")) };
        if status == 0 {
            self.id = None;
            Ok(())
        } else {
            Err(HostError::status("destroy script", status))
        }
    }

    /// Capture immutable values through the same initialized scene session.
    pub fn capture_scene_snapshot(
        &self,
        owner_file_ids: &[i64],
    ) -> Result<SceneSnapshot, HostError> {
        let required = required_size(owner_file_ids.len())?;
        let capture = self.host.scene_snapshot.ok_or_else(|| {
            HostError::setup(
                "capture scene snapshot",
                "snapshot capability was not loaded",
            )
        })?;
        let mut payload = vec![0_u8; required];
        let count = i32::try_from(owner_file_ids.len())
            .map_err(|_| HostError::status("capture scene snapshot", -2))?;
        let capacity = i32::try_from(payload.len())
            .map_err(|_| HostError::status("capture scene snapshot", -2))?;

        // SAFETY: the capability owns the trusted ABI; both borrowed buffers
        // remain valid and non-overlapping during this synchronous call.
        let status = unsafe {
            capture(
                self.id.expect("owned session"),
                owner_file_ids.as_ptr(),
                count,
                payload.as_mut_ptr(),
                capacity,
            )
        };
        if status != 0 {
            return Err(map_capture_status(status));
        }
        decode(&payload, owner_file_ids)
    }
}

fn map_capture_status(status: i32) -> HostError {
    if matches!(
        status,
        -1 | -2
            | -5
            | -6
            | -7
            | -101
            | -102
            | -103
            | -105
            | -107
            | -108
            | -109
            | -112
            | -113
            | -114
            | -115
            | -117
    ) {
        HostError::status("capture scene snapshot", status)
    } else {
        HostError::status("capture scene snapshot", -6)
    }
}

#[cfg(test)]
mod tests {
    use super::map_capture_status;

    #[test]
    fn forwards_only_defined_capture_statuses() {
        let mapped_statuses = [
            -1, -2, -5, -6, -7, -101, -102, -103, -105, -107, -108, -109, -112, -113, -114, -115,
            -117,
        ];

        for status in mapped_statuses {
            assert_eq!(map_capture_status(status).code(), Some(status));
        }

        assert_eq!(map_capture_status(-104).code(), Some(-6));
        assert_eq!(map_capture_status(-100).code(), Some(-6));
    }
}

impl Drop for ScriptSession<'_> {
    fn drop(&mut self) {
        if let Some(id) = self.id.take() {
            // SAFETY: unique live handle, trusted bridge, host outlives the session.
            let status = unsafe { (self.host.destroy)(id) };
            if status != 0 {
                // Cleanup diagnostics are best effort: stderr failure must not
                // replace an initiating failure or panic during unwinding.
                let _ = writeln!(
                    std::io::stderr().lock(),
                    "managed session cleanup failed: {status}"
                );
            }
        }
    }
}

#[cfg(all(test, feature = "conformance"))]
mod conformance;
