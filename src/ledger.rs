//! One private, atomically replaced file per run; no command arguments or output are stored.
use crate::carbon::Plan;
use anyhow::{Context, Result, ensure};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};
static SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Queued,
    Starting,
    Running,
    Completed,
    Cancelled,
    Failed,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Record {
    pub version: u8,
    pub id: String,
    pub program: String,
    pub status: Status,
    pub queued_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    pub exit_code: Option<i32>,
    pub plan: Option<Plan>,
    pub fallback_reason: Option<String>,
    pub failure_reason: Option<String>,
    pub launch_average_g_per_kwh: Option<f64>,
    pub avoided_g_per_kwh: Option<f64>,
    pub energy_kwh: Option<f64>,
    pub estimated_grams: Option<f64>,
}
impl Record {
    pub fn new(program: &str, plan: Option<Plan>, fallback_reason: Option<String>) -> Self {
        let queued_at = Utc::now();
        let id = format!(
            "{:020}-{}-{}",
            queued_at.timestamp_nanos_opt().unwrap_or(0),
            std::process::id(),
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
        );
        Self {
            version: 1,
            id,
            program: program
                .chars()
                .filter(|c| !c.is_control())
                .take(100)
                .collect(),
            status: Status::Queued,
            queued_at,
            started_at: None,
            finished_at: None,
            exit_code: None,
            plan,
            fallback_reason,
            failure_reason: None,
            launch_average_g_per_kwh: None,
            avoided_g_per_kwh: None,
            energy_kwh: None,
            estimated_grams: None,
        }
    }
    pub fn launched(&mut self, average: Option<f64>) {
        self.status = Status::Running;
        self.started_at = Some(Utc::now());
        self.launch_average_g_per_kwh = average;
        if let (Some(plan), Some(average)) = (&self.plan, average) {
            self.avoided_g_per_kwh = Some(plan.immediate.average_g_per_kwh - average);
            self.energy_kwh = plan.energy_kwh;
            self.estimated_grams = plan
                .energy_kwh
                .zip(self.avoided_g_per_kwh)
                .map(|(k, d)| k * d);
        }
    }
}
pub struct Ledger {
    directory: PathBuf,
}
impl Ledger {
    pub fn environment() -> Result<Self> {
        let directory = if let Some(p) = std::env::var_os("GRIDTOP_STATE_DIR") {
            PathBuf::from(p)
        } else if let Some(p) = std::env::var_os("XDG_STATE_HOME") {
            PathBuf::from(p).join("gridtop")
        } else {
            PathBuf::from(
                std::env::var_os("HOME").context("HOME or GRIDTOP_STATE_DIR is required")?,
            )
            .join(".local/state/gridtop")
        };
        Self::at(directory)
    }
    pub fn at(directory: PathBuf) -> Result<Self> {
        ensure!(directory.is_absolute(), "state directory must be absolute");
        Ok(Self { directory })
    }
    pub fn directory(&self) -> &Path {
        &self.directory
    }
    pub fn save(&self, record: &Record) -> Result<()> {
        use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
        ensure!(
            record.version == 1
                && !record.id.is_empty()
                && record.id.bytes().all(|c| c.is_ascii_digit() || c == b'-'),
            "invalid run record id/version"
        );
        let directory = self.directory.join("runs");
        fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(&directory)
            .context("cannot create local run ledger")?;
        let path = directory.join(format!("{}.json", record.id));
        let temp = directory.join(format!(
            ".{}-{}.tmp",
            record.id,
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        let result = (|| -> Result<()> {
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(&temp)?;
            file.write_all(&serde_json::to_vec_pretty(record)?)?;
            file.sync_all()?;
            fs::rename(&temp, &path)?;
            fs::File::open(&directory)?.sync_all()?;
            Ok(())
        })();
        if result.is_err() {
            let _ = fs::remove_file(&temp);
        }
        result.with_context(|| format!("cannot write local run ledger {}", path.display()))
    }
    pub fn list(&self, limit: usize) -> Result<Vec<Record>> {
        let directory = self.directory.join("runs");
        let entries = match fs::read_dir(&directory) {
            Ok(v) => v,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(e.into()),
        };
        let mut paths = Vec::new();
        for e in entries {
            let e = e?;
            if e.file_type()?.is_file() && e.path().extension().is_some_and(|x| x == "json") {
                paths.push(e.path());
            }
        }
        paths.sort();
        paths.reverse();
        paths
            .into_iter()
            .take(limit)
            .map(|p| {
                let mut bytes = Vec::new();
                fs::File::open(&p)?.take(65_537).read_to_end(&mut bytes)?;
                ensure!(
                    bytes.len() <= 65_536,
                    "ledger record exceeds 64 KiB: {}",
                    p.display()
                );
                let r: Record = serde_json::from_slice(&bytes)
                    .with_context(|| format!("invalid ledger record {}", p.display()))?;
                ensure!(r.version == 1, "unsupported ledger schema {}", r.version);
                Ok(r)
            })
            .collect()
    }
}
