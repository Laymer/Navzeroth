//! Versioned bounded inherited-pipe protocol. No sockets, command shells or raw
//! source paths in events. The worker seals research artifacts, never activates.
use crate::{
    Result,
    isolation::{ProcessLimit, worker_limit},
};
use azeroth_core::{local_data::PerformancePreset, local_data_catalog::ExtractionSummary};
use serde::{Deserialize, Serialize};
use std::{
    io::{Read, Write},
    path::PathBuf,
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
    sync::{Arc, Mutex},
};

pub const WORKER_ARGUMENT: &str = "--navzeroth-local-data-worker-v1";
const MAX_FRAME: usize = 64 * 1024;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExtractionRequest {
    pub protocol_version: u32,
    pub request_id: String,
    pub installation: PathBuf,
    pub product: String,
    pub destination: PathBuf,
    pub preset: PerformancePreset,
    /// Empty means every reviewed table candidate, not every game asset.
    pub tables: Vec<String>,
}
impl ExtractionRequest {
    pub fn validate(&self) -> Result<()> {
        if self.protocol_version != 1
            || self.request_id.is_empty()
            || self.request_id.len() > 64
            || !self
                .request_id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-')
            || self.tables.len() > 16_384
            || self.tables.iter().any(|t| {
                t.is_empty()
                    || t.len() > 80
                    || !t.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
            })
            || !matches!(self.product.as_str(), "wow_classic_beta")
        {
            return Err("Unsupported worker request");
        }
        let unique: std::collections::BTreeSet<_> = self.tables.iter().collect();
        if unique.len() != self.tables.len() {
            return Err("Duplicate requested tables");
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "event", rename_all = "snake_case", deny_unknown_fields)]
pub enum WorkerEvent {
    Progress {
        protocol_version: u32,
        request_id: String,
        completed: u32,
        total: u32,
    },
    Complete {
        protocol_version: u32,
        request_id: String,
        summary: ExtractionSummary,
    },
    Failed {
        protocol_version: u32,
        request_id: String,
        code: String,
    },
}

fn write_frame<T: Serialize>(writer: &mut impl Write, value: &T) -> Result<()> {
    let bytes = serde_json::to_vec(value).map_err(|_| "Worker encoding failed")?;
    if bytes.len() > MAX_FRAME {
        return Err("Worker frame exceeds capacity");
    }
    writer
        .write_all(&(bytes.len() as u32).to_le_bytes())
        .and_then(|()| writer.write_all(&bytes))
        .and_then(|()| writer.flush())
        .map_err(|_| "Worker pipe unavailable")
}
fn read_frame<T: serde::de::DeserializeOwned>(reader: &mut impl Read) -> Result<T> {
    let mut size = [0; 4];
    reader
        .read_exact(&mut size)
        .map_err(|_| "Worker pipe closed")?;
    let size = u32::from_le_bytes(size) as usize;
    if size > MAX_FRAME {
        return Err("Worker frame exceeds capacity");
    }
    let mut bytes = vec![0; size];
    reader
        .read_exact(&mut bytes)
        .map_err(|_| "Truncated worker frame")?;
    serde_json::from_value(azeroth_core::strict_json::value(&bytes, MAX_FRAME)?)
        .map_err(|_| "Invalid worker frame")
}

pub fn entry() -> i32 {
    // Parent attaches OS limits before writing the first bounded request.
    let request: ExtractionRequest = match read_frame(&mut std::io::stdin().lock()) {
        Ok(v) => v,
        Err(_) => return 2,
    };
    let operation = request
        .validate()
        .and_then(|()| worker_limit(request.preset.limits().worker_memory_bytes))
        .and_then(|()| {
            crate::extraction::extract(&request, |completed, total| {
                write_frame(
                    &mut std::io::stdout().lock(),
                    &WorkerEvent::Progress {
                        protocol_version: 1,
                        request_id: request.request_id.clone(),
                        completed,
                        total,
                    },
                )
            })
        });
    let (event, exit) = match operation {
        Ok(summary) => (
            WorkerEvent::Complete {
                protocol_version: 1,
                request_id: request.request_id,
                summary,
            },
            0,
        ),
        Err(code) => (
            WorkerEvent::Failed {
                protocol_version: 1,
                request_id: request.request_id,
                code: code.into(),
            },
            1,
        ),
    };
    if write_frame(&mut std::io::stdout().lock(), &event).is_err() {
        return 1;
    }
    exit
}

pub struct ExtractionWorker {
    child: Arc<Mutex<Child>>,
    output: ChildStdout,
    input: Option<ChildStdin>,
    _limit: ProcessLimit,
    request_id: String,
    terminal: bool,
}
#[derive(Clone)]
pub struct WorkerControl(Arc<Mutex<Child>>);
impl WorkerControl {
    pub fn cancel(&self) -> Result<()> {
        let mut child = self.0.lock().map_err(|_| "Worker control unavailable")?;
        let running = child
            .try_wait()
            .map_err(|_| "Cannot inspect worker state")?
            .is_none();
        if running
            && child.kill().is_err()
            && child
                .try_wait()
                .map_err(|_| "Cannot inspect worker state")?
                .is_none()
        {
            return Err("Cannot cancel extraction worker");
        }
        let _ = child.wait();
        Ok(())
    }
}
impl ExtractionWorker {
    pub fn start(request: &ExtractionRequest) -> Result<Self> {
        request.validate()?;
        let exe = std::env::current_exe().map_err(|_| "Cannot locate extraction executable")?;
        Self::start_executable(&exe, request)
    }
    pub fn start_executable(
        executable: &std::path::Path,
        request: &ExtractionRequest,
    ) -> Result<Self> {
        request.validate()?;
        let mut child = Command::new(executable)
            .arg(WORKER_ARGUMENT)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|_| "Cannot start extraction worker")?;
        let limit = match ProcessLimit::attach(&child, request.preset.limits().worker_memory_bytes)
        {
            Ok(v) => v,
            Err(e) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(e);
            }
        };
        let input = child.stdin.take().ok_or("Worker input unavailable")?;
        let output = child.stdout.take().ok_or("Worker output unavailable")?;
        let mut worker = Self {
            child: Arc::new(Mutex::new(child)),
            output,
            input: Some(input),
            _limit: limit,
            request_id: request.request_id.clone(),
            terminal: false,
        };
        write_frame(worker.input.as_mut().unwrap(), request)?;
        Ok(worker)
    }
    pub fn next_event(&mut self) -> Result<WorkerEvent> {
        if self.terminal {
            return Err("Worker already terminated");
        }
        let event: WorkerEvent = read_frame(&mut self.output)?;
        let (version, id, terminal) = match &event {
            WorkerEvent::Progress {
                protocol_version,
                request_id,
                ..
            } => (*protocol_version, request_id, false),
            WorkerEvent::Complete {
                protocol_version,
                request_id,
                ..
            }
            | WorkerEvent::Failed {
                protocol_version,
                request_id,
                ..
            } => (*protocol_version, request_id, true),
        };
        if version != 1 || id != &self.request_id {
            return Err("Worker response identity mismatch");
        }
        if terminal {
            self.input.take();
            let status = self
                .child
                .lock()
                .map_err(|_| "Worker control unavailable")?
                .wait()
                .map_err(|_| "Cannot reap extraction worker")?;
            if matches!(event, WorkerEvent::Complete { .. }) && !status.success() {
                return Err("Extraction worker crashed");
            }
            self.terminal = true;
        }
        Ok(event)
    }
    /// Hard cancellation terminates native work and releases OS locks/limits.
    /// Unsealed objects may remain; no active catalog is modified by extraction.
    pub fn cancel(&mut self) -> Result<()> {
        if !self.terminal {
            self.control().cancel()?;
            self.terminal = true;
        }
        Ok(())
    }
    pub fn control(&self) -> WorkerControl {
        WorkerControl(self.child.clone())
    }
}
impl Drop for ExtractionWorker {
    fn drop(&mut self) {
        let _ = self.cancel();
    }
}
