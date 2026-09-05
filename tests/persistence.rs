#![cfg(feature = "nedb")]
use forge_ui::{journal::Journal, *};
use serde_json::json;
use std::time::{SystemTime, UNIX_EPOCH};
fn path(label: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "forge-{label}-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ))
}
fn object_files(dir: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut files = vec![];
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            files.extend(object_files(&path));
        } else {
            files.push(path);
        }
    }
    files
}
#[test]
fn real_nedb_reopen_history_and_as_of() {
    let path = path("persist");
    let a = Action {
        id: "counter".into(),
        kind: ActionKind::Activate,
    };
    let (first, second) = {
        let j = Journal::open(&path).unwrap();
        assert!(j.load().is_none());
        let one = j.commit(json!({"count":1}), &a, Source::Human).unwrap();
        let two = j.commit(json!({"count":2}), &a, Source::Agent).unwrap();
        assert_ne!(one.hash, two.hash);
        assert!(j.verify().unwrap() >= 2);
        (one, two)
    };
    let j = Journal::open(&path).unwrap();
    assert_eq!(j.load().unwrap()["count"], 2);
    assert_eq!(j.latest().unwrap().hash, second.hash);
    assert_eq!(j.state_as_of(first.seq).unwrap()["count"], 1);
    let history = j.history(10);
    assert_eq!(history.len(), 2);
    assert_eq!(history[0]["caused_by"][0], first.hash);
    assert_eq!(history[0]["data"]["source"], "agent");
    let old_files: Vec<_> = object_files(&path.join("objects"))
        .into_iter()
        .map(|p| {
            let b = std::fs::read(&p).unwrap();
            (p, b)
        })
        .collect();
    j.commit(
        j.state_as_of(first.seq).unwrap(),
        &Action {
            id: "restore".into(),
            kind: ActionKind::Activate,
        },
        Source::Human,
    )
    .unwrap();
    assert_eq!(j.load().unwrap()["count"], 1);
    assert_eq!(j.history(10).len(), 3);
    for (p, b) in old_files {
        assert_eq!(std::fs::read(p).unwrap(), b);
    }
}
#[test]
fn exclusive_store_lock() {
    let path = path("lock");
    let j = Journal::open(&path).unwrap();
    assert!(Journal::open(&path).is_err());
    drop(j);
    assert!(Journal::open(&path).is_ok());
}
#[test]
fn corruption_is_reported_without_repair() {
    let path = path("corrupt");
    {
        let j = Journal::open(&path).unwrap();
        j.commit(
            json!({"count":1}),
            &Action {
                id: "a".into(),
                kind: ActionKind::Activate,
            },
            Source::Human,
        )
        .unwrap();
    }
    let object = object_files(&path.join("objects"))
        .into_iter()
        .next()
        .unwrap();
    let mut b = std::fs::read(&object).unwrap();
    let mid = b.len() / 2;
    b[mid] ^= 1;
    std::fs::write(&object, &b).unwrap();
    assert!(Journal::open(&path).is_err());
    assert_eq!(std::fs::read(&object).unwrap(), b);
}
