use crate::state::State;
use forge_ui::{
    journal::{Journal, Receipt},
    *,
};
use serde_json::{json, Value};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

pub struct Desk {
    pub state: State,
    pub receipt: Option<Receipt>,
    pub history: Vec<Value>,
    pub status: String,
    pub error: Option<String>,
    pub storage_blocked: bool,
    pub verified: usize,
    pub data_path: PathBuf,
    journal: Journal,
}
impl Desk {
    pub fn open(path: &Path) -> Result<Self, String> {
        // A dedicated app marker avoids accidentally repurposing a framework showcase
        // or another application's database passed via --data.
        fs::create_dir_all(path).map_err(|e| e.to_string())?;
        let marker = path.join("forge-desk.store");
        if marker.exists() {
            if fs::read(&marker).map_err(|e| e.to_string())? != b"forge-desk/v1\n" {
                return Err("Unrecognized store marker; no database opened".into());
            }
        } else {
            if fs::read_dir(path)
                .map_err(|e| e.to_string())?
                .next()
                .is_some()
            {
                return Err("Directory is not empty and is not a Forge Desk store. Use a NEW dedicated --data directory; existing files were not changed.".into());
            }
            let mut f = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(marker)
                .map_err(|e| e.to_string())?;
            f.write_all(b"forge-desk/v1\n").map_err(|e| e.to_string())?;
            f.sync_all().map_err(|e| e.to_string())?;
        }
        let journal = Journal::open(path)?;
        let state: State = match journal.load() {
            Some(v) => serde_json::from_value(v)
                .map_err(|e| format!("State format mismatch: {e}. Original state preserved."))?,
            None => State::default(),
        };
        state.validate()?;
        let verified = journal.verify()?;
        let receipt = journal.latest();
        let history = journal.history(6);
        Ok(Self {
            state,
            receipt,
            history,
            status: "Ready. Your next task starts here.".into(),
            error: None,
            storage_blocked: false,
            verified,
            data_path: path.into(),
            journal,
        })
    }
    pub fn act(&mut self, action: Action, source: Source) {
        if self.storage_blocked {
            return;
        }
        if action.id == "verify" && action.kind == ActionKind::Activate {
            match self.journal.verify() {
                Ok(n) => {
                    self.verified = n;
                    self.status = format!("Integrity verified / {n} stored objects");
                    self.error = None;
                }
                Err(e) => {
                    self.error = Some(e);
                    self.storage_blocked = true;
                }
            }
            return;
        }
        let candidate = match self.state.reduce(&action) {
            Ok(s) => s,
            Err(e) => {
                self.error = Some(e);
                return;
            }
        };
        if candidate == self.state {
            self.error = None;
            return;
        }
        let value = match serde_json::to_value(&candidate) {
            Ok(v) => v,
            Err(e) => {
                self.error = Some(e.to_string());
                return;
            }
        };
        match self.journal.commit(value, &action, source) {
            Ok(receipt) => {
                self.status = format!("Saved / {} / seq {} / {:?}", action.id, receipt.seq, source);
                self.receipt = Some(receipt);
                self.state = candidate;
                self.error = None;
                self.history = self.journal.history(6);
            }
            Err(e) => {
                self.storage_blocked = true;
                self.error=Some(format!("Write not acknowledged: {e}. Writes paused. Close and reopen to inspect the store."));
            }
        }
    }
}
impl Application for Desk {
    fn view(&self) -> Node {
        crate::view::build(self)
    }
    fn update(&mut self, a: Action) {
        self.act(a, Source::Human);
    }
    fn update_from(&mut self, a: Action, source: Source) {
        self.act(a, source);
    }
    fn theme(&self) -> Theme {
        if self.state.light {
            Theme::LIGHT
        } else {
            Theme::DARK
        }
    }
    fn inspect(&self) -> Value {
        json!({"app":"forge-desk","state":self.state,"receipt":self.receipt,"error":self.error,"storage_blocked":self.storage_blocked,"verified_objects":self.verified,"history":self.history})
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn temp(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "forge-desk-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }
    #[test]
    fn save_reopen_real_journal() {
        let path = temp("restart");
        let hash;
        {
            let mut d = Desk::open(&path).unwrap();
            d.act(
                Action {
                    id: "draft".into(),
                    kind: ActionKind::ChangeText("Ship starter".into()),
                },
                Source::Agent,
            );
            d.act(
                Action {
                    id: "add".into(),
                    kind: ActionKind::Activate,
                },
                Source::Agent,
            );
            assert!(d.error.is_none());
            hash = d.receipt.unwrap().hash;
        }
        let d = Desk::open(&path).unwrap();
        assert_eq!(d.state.tasks[0].title, "Ship starter");
        assert_eq!(d.receipt.as_ref().unwrap().hash, hash);
        assert!(d.verified >= 2);
        assert_eq!(d.history[0]["data"]["source"], "agent");
    }
    #[test]
    fn refuses_unrelated_directory_without_changes() {
        let path = temp("unrelated");
        fs::create_dir_all(&path).unwrap();
        fs::write(path.join("user-data"), b"preserve exactly").unwrap();
        assert!(Desk::open(&path).is_err());
        assert_eq!(
            fs::read(path.join("user-data")).unwrap(),
            b"preserve exactly"
        );
        assert!(!path.join("forge-desk.store").exists());
    }
    #[test]
    fn invalid_input_not_committed() {
        let path = temp("bad-input");
        let mut d = Desk::open(&path).unwrap();
        d.act(
            Action {
                id: "add".into(),
                kind: ActionKind::Activate,
            },
            Source::Agent,
        );
        assert!(d.error.is_some());
        assert!(d.receipt.is_none());
        assert!(d.journal.load().is_none());
    }
    #[test]
    fn ui_tree_uses_stable_task_ids() {
        let path = temp("view");
        let mut d = Desk::open(&path).unwrap();
        d.act(
            Action {
                id: "draft".into(),
                kind: ActionKind::ChangeText("First".into()),
            },
            Source::Human,
        );
        d.act(
            Action {
                id: "add".into(),
                kind: ActionKind::Activate,
            },
            Source::Human,
        );
        let mut ui = Ui::new(d.view()).unwrap();
        ui.resize(1080, 760, 1.0);
        ui.paint();
        assert!(ui.item("task.1").is_some());
        assert!(ui.item("task.title.1").is_some());
    }
}
