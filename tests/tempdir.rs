//! Minimal scoped temp-dir RAII for tests.
pub struct TempDir(pub std::path::PathBuf);
impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
