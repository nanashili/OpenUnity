use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static NEXT_WORKSPACE_ORDINAL: AtomicU64 = AtomicU64::new(0);

pub(crate) struct Workspace {
    pub(crate) root: PathBuf,
}

impl Workspace {
    pub(crate) fn new() -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock must be after the Unix epoch")
            .as_nanos();
        let ordinal = NEXT_WORKSPACE_ORDINAL.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "openunity-wav-mix-{}-{nonce}-{ordinal}",
            std::process::id()
        ));
        fs::create_dir(&root).expect("create unique workspace");
        Self { root }
    }

    pub(crate) fn path(&self, name: &str) -> PathBuf {
        self.root.join(name)
    }

    pub(crate) fn write(&self, name: &str, bytes: &[u8]) -> PathBuf {
        let path = self.path(name);
        fs::write(&path, bytes).expect("write source WAVE");
        path
    }
}

impl Drop for Workspace {
    fn drop(&mut self) {
        if let Err(error) = fs::remove_dir_all(&self.root) {
            if std::thread::panicking() {
                eprintln!(
                    "failed to remove {} while unwinding: {error}",
                    self.root.display()
                );
            } else {
                panic!("failed to remove {}: {error}", self.root.display());
            }
        }
    }
}
