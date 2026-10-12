use super::ManagedHost;
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
