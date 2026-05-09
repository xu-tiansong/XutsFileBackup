use std::path::Path;

use crate::error::AppResult;

pub struct SyncStats {
    pub files_copied: u64,
    pub files_deleted: u64,
    pub bytes_transferred: u64,
    pub errors: Vec<String>,
}

impl Default for SyncStats {
    fn default() -> Self {
        Self { files_copied: 0, files_deleted: 0, bytes_transferred: 0, errors: Vec::new() }
    }
}

pub fn copy_file(source_base: &Path, target_base: &Path, relative_path: &str) -> Result<u64, String> {
    let rel_os = relative_path.replace('/', std::path::MAIN_SEPARATOR_STR);
    let src = source_base.join(&rel_os);
    let dst = target_base.join(&rel_os);

    if let Some(parent) = dst.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("mkdir '{}': {}", relative_path, e))?;
    }

    std::fs::copy(&src, &dst)
        .map_err(|e| format!("copy '{}': {}", relative_path, e))
}

pub fn delete_file(target_base: &Path, relative_path: &str) -> Result<(), String> {
    let rel_os = relative_path.replace('/', std::path::MAIN_SEPARATOR_STR);
    let dst = target_base.join(&rel_os);

    match std::fs::remove_file(&dst) {
        Ok(_) => Ok(()),
        // Already gone is acceptable in mirror mode
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(format!("delete '{}': {}", relative_path, e)),
    }
}

/// Hash a source file with xxHash3-64. Returns a 16-char hex string.
pub fn hash_file(source_base: &Path, relative_path: &str) -> AppResult<String> {
    let rel_os = relative_path.replace('/', std::path::MAIN_SEPARATOR_STR);
    let data = std::fs::read(source_base.join(rel_os))?;
    Ok(format!("{:016x}", xxhash_rust::xxh3::xxh3_64(&data)))
}
