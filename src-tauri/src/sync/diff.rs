use std::collections::HashMap;

use crate::sync::scanner::ScanMap;

pub struct DbEntry {
    pub id: i64,
    pub size: u64,
    pub mtime_secs: i64,
    pub content_hash: String,
}

pub type DbMap = HashMap<String, DbEntry>;

pub enum SyncAction {
    Add    { relative_path: String, size: u64, mtime_secs: i64 },
    Update { file_id: i64, relative_path: String, size: u64, mtime_secs: i64 },
    Delete { file_id: i64, relative_path: String },
}

pub fn compute_diff(scan: &ScanMap, db: &DbMap) -> Vec<SyncAction> {
    let mut actions = Vec::new();

    for (path, entry) in scan {
        match db.get(path) {
            None => actions.push(SyncAction::Add {
                relative_path: path.clone(),
                size: entry.size,
                mtime_secs: entry.mtime_secs,
            }),
            Some(db_entry) => {
                if entry.size != db_entry.size || entry.mtime_secs != db_entry.mtime_secs {
                    actions.push(SyncAction::Update {
                        file_id: db_entry.id,
                        relative_path: path.clone(),
                        size: entry.size,
                        mtime_secs: entry.mtime_secs,
                    });
                }
            }
        }
    }

    for (path, db_entry) in db {
        if !scan.contains_key(path) {
            actions.push(SyncAction::Delete {
                file_id: db_entry.id,
                relative_path: path.clone(),
            });
        }
    }

    actions
}
