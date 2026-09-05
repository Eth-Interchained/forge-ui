use forge_ui::{Action, ActionKind};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

pub const MAX_TASKS: usize = 200;
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Task {
    pub id: u64,
    pub title: String,
    pub done: bool,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Filter {
    #[default]
    All,
    Open,
    Done,
}
impl Filter {
    pub fn includes(self, task: &Task) -> bool {
        match self {
            Self::All => true,
            Self::Open => !task.done,
            Self::Done => task.done,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct State {
    pub app: String,
    pub schema: u32,
    pub next_id: u64,
    pub tasks: Vec<Task>,
    pub draft: String,
    pub light: bool,
    pub filter: Filter,
}
impl Default for State {
    fn default() -> Self {
        Self {
            app: "forge-desk".into(),
            schema: 1,
            next_id: 1,
            tasks: vec![],
            draft: String::new(),
            light: false,
            filter: Filter::All,
        }
    }
}
impl State {
    pub fn validate(&self) -> Result<(), String> {
        if self.app != "forge-desk" || self.schema != 1 {
            return Err("This store does not contain supported Forge Desk state. Select a new --data directory; original data is untouched.".into());
        }
        if self.tasks.len() > MAX_TASKS || !valid_text(&self.draft) {
            return Err("Invalid stored task count or draft".into());
        }
        let mut ids = HashSet::new();
        for t in &self.tasks {
            if t.id == 0
                || t.id >= self.next_id
                || !ids.insert(t.id)
                || t.title.trim().is_empty()
                || !valid_text(&t.title)
            {
                return Err("Invalid stored task identity or title".into());
            }
        }
        if self.next_id == 0 {
            return Err("Invalid task sequence".into());
        }
        Ok(())
    }
    pub fn can_add(&self) -> bool {
        !self.draft.trim().is_empty() && self.tasks.len() < MAX_TASKS && self.next_id < u64::MAX
    }
    /// Pure reducer. No database, UI or I/O assumptions. Unknown actions are rejected.
    pub fn reduce(&self, a: &Action) -> Result<Self, String> {
        let mut s = self.clone();
        match (a.id.as_str(), &a.kind) {
            ("draft", ActionKind::ChangeText(text)) => {
                if !valid_text(text) {
                    return Err("Use a single-line task of at most 120 UTF-8 bytes.".into());
                }
                s.draft = text.clone();
            }
            ("add", ActionKind::Activate) | ("draft", ActionKind::Submit(_)) => {
                if !s.can_add() {
                    return Err("Enter a task first. This starter supports up to 200 tasks.".into());
                }
                s.tasks.push(Task {
                    id: s.next_id,
                    title: s.draft.trim().into(),
                    done: false,
                });
                s.next_id += 1;
                s.draft.clear();
                s.filter = Filter::All;
            }
            ("theme", ActionKind::Toggle(value)) => s.light = *value,
            ("filter.all", ActionKind::Activate) => s.filter = Filter::All,
            ("filter.open", ActionKind::Activate) => s.filter = Filter::Open,
            ("filter.done", ActionKind::Activate) => s.filter = Filter::Done,
            (id, ActionKind::Toggle(value)) if id.starts_with("task.") => {
                let id: u64 = id
                    .strip_prefix("task.")
                    .unwrap()
                    .parse()
                    .map_err(|_| "Invalid task ID")?;
                let task = s
                    .tasks
                    .iter_mut()
                    .find(|t| t.id == id)
                    .ok_or("Unknown task ID")?;
                task.done = *value;
            }
            _ => return Err("Unsupported action".into()),
        }
        s.validate()?;
        Ok(s)
    }
}
fn valid_text(s: &str) -> bool {
    s.len() <= 120 && !s.chars().any(char::is_control)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn action(id: &str, kind: ActionKind) -> Action {
        Action {
            id: id.into(),
            kind,
        }
    }
    #[test]
    fn empty_add_rejected() {
        assert!(State::default()
            .reduce(&action("add", ActionKind::Activate))
            .is_err());
    }
    #[test]
    fn add_trims_and_clears_draft() {
        let s = State {
            draft: "  Build a thing  ".into(),
            ..State::default()
        }
        .reduce(&action("add", ActionKind::Activate))
        .unwrap();
        assert_eq!(s.tasks[0].title, "Build a thing");
        assert_eq!(s.tasks[0].id, 1);
        assert_eq!(s.next_id, 2);
        assert!(s.draft.is_empty());
    }
    #[test]
    fn completion_preserves_identity() {
        let s = State {
            draft: "Task".into(),
            ..State::default()
        }
        .reduce(&action("add", ActionKind::Activate))
        .unwrap();
        let next = s
            .reduce(&action("task.1", ActionKind::Toggle(true)))
            .unwrap();
        assert_eq!(next.tasks[0].id, s.tasks[0].id);
        assert!(next.tasks[0].done);
        assert!(!s.tasks[0].done);
    }
    #[test]
    fn wrong_identity_schema_and_duplicate_ids_rejected() {
        let mut s = State {
            app: "other-app".into(),
            ..State::default()
        };
        assert!(s.validate().is_err());
        s = State {
            schema: 2,
            ..State::default()
        };
        assert!(s.validate().is_err());
        s = State {
            draft: "X".into(),
            ..State::default()
        }
        .reduce(&action("add", ActionKind::Activate))
        .unwrap();
        s.tasks.push(s.tasks[0].clone());
        assert!(s.validate().is_err());
    }
    #[test]
    fn multiline_and_oversize_rejected() {
        for t in ["a\nb".to_string(), "a".repeat(121)] {
            assert!(State::default()
                .reduce(&action("draft", ActionKind::ChangeText(t)))
                .is_err());
        }
    }
    #[test]
    fn filters_are_pure() {
        let t = Task {
            id: 1,
            title: "X".into(),
            done: true,
        };
        assert!(Filter::All.includes(&t));
        assert!(Filter::Done.includes(&t));
        assert!(!Filter::Open.includes(&t));
    }
    #[test]
    fn wrong_action_has_no_effect() {
        let s = State::default();
        assert!(s.reduce(&action("theme", ActionKind::Activate)).is_err());
        assert!(s
            .reduce(&action("task.999", ActionKind::Toggle(true)))
            .is_err());
        assert_eq!(s, State::default());
    }
    #[test]
    fn cap_prevents_unbounded_snapshot_growth() {
        let s = State {
            tasks: (1..=200)
                .map(|id| Task {
                    id,
                    title: "X".into(),
                    done: false,
                })
                .collect(),
            next_id: 201,
            draft: "more".into(),
            ..State::default()
        };
        assert!(!s.can_add());
        assert!(s.reduce(&action("add", ActionKind::Activate)).is_err());
    }
}
