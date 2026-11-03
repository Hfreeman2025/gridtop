use gridtop::ledger::{Ledger, Record, Status};
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
static N: AtomicU64 = AtomicU64::new(0);
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join(".tmp")
            .join(format!(
                "ledger-test-{}-{}",
                std::process::id(),
                N.fetch_add(1, Ordering::Relaxed)
            ));
        fs::create_dir_all(&p).unwrap();
        Self(p)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
#[test]
fn ledger_is_durable_private_atomic_and_contains_no_arguments_or_outputs() {
    use std::os::unix::fs::PermissionsExt;
    let t = Temp::new();
    let l = Ledger::at(t.0.clone()).unwrap();
    assert!(l.list(20).unwrap().is_empty());
    let mut r = Record::new("printf", None, Some("fixture: unavailable".into()));
    l.save(&r).unwrap();
    assert_eq!(l.list(20).unwrap()[0].status, Status::Queued);
    r.launched(None);
    r.status = Status::Completed;
    r.exit_code = Some(17);
    l.save(&r).unwrap();
    let rows = l.list(20).unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].exit_code, Some(17));
    assert!(rows[0].avoided_g_per_kwh.is_none());
    assert!(rows[0].estimated_grams.is_none());
    let p = t.0.join("runs").join(format!("{}.json", r.id));
    assert_eq!(
        fs::metadata(&p).unwrap().permissions().mode() & 0o777,
        0o600
    );
    let text = fs::read_to_string(p).unwrap();
    assert!(!text.contains("arguments") && !text.contains("stdout"));
    assert_eq!(fs::read_dir(t.0.join("runs")).unwrap().count(), 1);
}
#[test]
fn corrupt_or_oversized_records_and_path_traversal_fail_explicitly() {
    let t = Temp::new();
    let l = Ledger::at(t.0.clone()).unwrap();
    let mut r = Record::new("true", None, None);
    r.id = "../escape".into();
    assert!(l.save(&r).is_err());
    fs::create_dir_all(t.0.join("runs")).unwrap();
    let p = t.0.join("runs/bad.json");
    fs::write(&p, "not json").unwrap();
    assert!(l.list(20).is_err());
    fs::write(&p, vec![b' '; 65_537]).unwrap();
    assert!(l.list(20).is_err());
    assert!(Ledger::at(PathBuf::from("relative")).is_err());
}
#[test]
fn concurrent_writers_do_not_lose_runs() {
    let t = Temp::new();
    std::thread::scope(|scope| {
        for _ in 0..8 {
            let p = t.0.clone();
            scope.spawn(move || {
                let l = Ledger::at(p).unwrap();
                l.save(&Record::new("true", None, None)).unwrap();
            });
        }
    });
    assert_eq!(Ledger::at(t.0.clone()).unwrap().list(20).unwrap().len(), 8);
}
