//! Optional real NEDB persistence adapter. One immutable version holds action + resulting state.
//! Restoration appends a new version; it never deletes or rewrites history.
use crate::{Action, Source};
use fs2::FileExt;
use nedb_engine::Db;
use serde_json::{json, Value};
use std::{fs::File, path::Path};

#[derive(Clone, Debug, serde::Serialize)]
pub struct Receipt {
    pub seq: u64,
    pub hash: String,
    pub head: String,
}
pub struct Journal {
    db: Db,
    _lock: File,
}
impl Journal {
    /// Exclusive process lock prevents two showcase processes sharing the same store.
    /// Existing data is never repaired, reset or deleted automatically.
    pub fn open(path: &Path) -> Result<Self, String> {
        std::fs::create_dir_all(path).map_err(|e| e.to_string())?;
        let lock = std::fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(path.join("forge.lock"))
            .map_err(|e| e.to_string())?;
        lock.try_lock_exclusive()
            .map_err(|e| format!("store already in use or unavailable: {e}"))?;
        let db = Db::open(path, None).map_err(|e| e.to_string())?;
        let (_, bad) = db.verify();
        if !bad.is_empty() {
            return Err(format!(
                "integrity check failed; original files preserved: {bad:?}"
            ));
        }
        Ok(Self { db, _lock: lock })
    }
    pub fn load(&self) -> Option<Value> {
        self.db
            .get("forge_state", "app")
            .map(|n| n.data["state"].clone())
    }
    pub fn latest(&self) -> Option<Receipt> {
        self.db.get("forge_state", "app").map(|n| Receipt {
            seq: n.seq,
            hash: n.hash,
            head: self.db.head(),
        })
    }
    pub fn commit(&self, state: Value, action: &Action, source: Source) -> Result<Receipt, String> {
        let parents = self
            .db
            .get("forge_state", "app")
            .map(|n| vec![n.hash])
            .unwrap_or_default();
        let node = self
            .db
            .put(
                "forge_state",
                "app",
                json!({"schema":1,"state":state,"action":action,"source":source}),
                parents,
                None,
                None,
            )
            .map_err(|e| e.to_string())?;
        // A successful put is not a durable acknowledgement. Both flushes must succeed.
        self.db
            .try_flush_all()
            .map_err(|e| format!("state not durably acknowledged: {e}"))?;
        self.db
            .try_flush_manifest()
            .map_err(|e| format!("manifest flush failed: {e}"))?;
        let read = self
            .db
            .get_by_hash(&node.hash)
            .ok_or("read-after-write integrity check failed")?;
        if read.data != node.data {
            return Err("read-after-write data mismatch".into());
        }
        Ok(Receipt {
            seq: node.seq,
            hash: node.hash,
            head: self.db.head(),
        })
    }
    pub fn history(&self, limit: usize) -> Vec<Value> {
        let Some(tip) = self.db.get("forge_state", "app") else {
            return vec![];
        };
        let mut nodes = self.db.trace(&tip.hash, false, limit);
        nodes.sort_by_key(|n| std::cmp::Reverse(n.seq));
        nodes
            .into_iter()
            .map(|n| json!({"seq":n.seq,"hash":n.hash,"caused_by":n.caused_by,"data":n.data}))
            .collect()
    }
    pub fn state_as_of(&self, seq: u64) -> Option<Value> {
        self.db
            .get_as_of("forge_state", "app", seq)
            .map(|n| n.data["state"].clone())
    }
    pub fn verify(&self) -> Result<usize, String> {
        let (count, bad) = self.db.verify();
        if bad.is_empty() {
            Ok(count)
        } else {
            Err(format!("integrity errors: {bad:?}"))
        }
    }
}
