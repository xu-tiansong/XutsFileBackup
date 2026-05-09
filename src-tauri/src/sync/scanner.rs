use std::collections::HashMap;
use std::path::Path;
use std::time::SystemTime;
use walkdir::WalkDir;

use crate::error::AppResult;

pub struct FileEntry {
    pub size: u64,
    pub mtime_secs: i64,
}

pub type ScanMap = HashMap<String, FileEntry>;

pub fn scan_directory(base: &Path, exclude_globs: &[String]) -> AppResult<ScanMap> {
    let mut map = ScanMap::new();

    for result in WalkDir::new(base).follow_links(false).into_iter() {
        let entry = match result {
            Ok(e) => e,
            Err(_) => continue,
        };

        if !entry.file_type().is_file() {
            continue;
        }

        let relative = match entry.path().strip_prefix(base) {
            Ok(rel) => rel.to_string_lossy().replace('\\', "/"),
            Err(_) => continue,
        };

        if is_excluded(&relative, exclude_globs) {
            continue;
        }

        let metadata = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };

        let mtime_secs = metadata
            .modified()
            .ok()
            .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        map.insert(relative, FileEntry { size: metadata.len(), mtime_secs });
    }

    Ok(map)
}

fn is_excluded(relative_path: &str, excludes: &[String]) -> bool {
    excludes.iter().any(|pattern| {
        let pat = pattern.trim_end_matches('/');
        relative_path == pat
            || relative_path.starts_with(&format!("{}/", pat))
            || relative_path.contains(&format!("/{}/", pat))
    })
}
