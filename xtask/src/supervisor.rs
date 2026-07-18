use serde::Serialize;
use std::ffi::{OsStr, OsString};
use std::io::{self, Read};
use std::path::PathBuf;
use std::process::{Command, ExitStatus, Stdio};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread;
use std::time::{Duration, Instant};

const POLL_INTERVAL: Duration = Duration::from_millis(10);
const READ_BUFFER_BYTES: usize = 8 * 1024;

#[derive(Debug, Clone)]
pub struct ProcessSpec {
    program: OsString,
    args: Vec<OsString>,
    current_dir: Option<PathBuf>,
    environment: Vec<(OsString, OsString)>,
    timeout: Duration,
    stdout_limit: usize,
    stderr_limit: usize,
}

impl ProcessSpec {
    pub fn new(program: impl Into<OsString>) -> Self {
        Self {
            program: program.into(),
            args: Vec::new(),
            current_dir: None,
            environment: Vec::new(),
            timeout: Duration::from_secs(60),
            stdout_limit: 8 * 1024 * 1024,
            stderr_limit: 8 * 1024 * 1024,
        }
    }

    pub fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        self.args
            .extend(args.into_iter().map(|value| value.as_ref().to_os_string()));
        self
    }

    pub fn current_dir(mut self, path: impl Into<PathBuf>) -> Self {
        self.current_dir = Some(path.into());
        self
    }

    pub fn env(mut self, key: impl Into<OsString>, value: impl Into<OsString>) -> Self {
        self.environment.push((key.into(), value.into()));
        self
    }

    pub const fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub const fn output_limits(mut self, stdout: usize, stderr: usize) -> Self {
        self.stdout_limit = stdout;
        self.stderr_limit = stderr;
        self
    }

    pub fn rendered_command(&self) -> Vec<String> {
        std::iter::once(self.program.to_string_lossy().into_owned())
            .chain(
                self.args
                    .iter()
                    .map(|argument| argument.to_string_lossy().into_owned()),
            )
            .collect()
    }
}

#[derive(Debug)]
pub struct ProcessOutput {
    pub status: ExitStatus,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub elapsed: Duration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProcessFailureKind {
    MissingExecutable,
    CouldNotStart,
    EnvironmentBlocked,
    TimedOut,
    OutputLimitExceeded,
    NonZeroExit,
    Io,
}

#[derive(Debug)]
pub struct ProcessFailure {
    pub kind: ProcessFailureKind,
    pub exit_code: Option<i32>,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub elapsed: Duration,
}

impl ProcessFailure {
    fn without_child(kind: ProcessFailureKind, elapsed: Duration) -> Self {
        Self {
            kind,
            exit_code: None,
            stdout: Vec::new(),
            stderr: Vec::new(),
            elapsed,
        }
    }
}

struct CapturedStream {
    bytes: Vec<u8>,
    exceeded: bool,
}

pub fn run(spec: &ProcessSpec) -> Result<ProcessOutput, ProcessFailure> {
    let started = Instant::now();
    let mut command = Command::new(&spec.program);
    command
        .args(&spec.args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(current_dir) = &spec.current_dir {
        command.current_dir(current_dir);
    }
    for (key, value) in &spec.environment {
        command.env(key, value);
    }

    let mut child = command.spawn().map_err(|error| {
        ProcessFailure::without_child(classify_spawn_error(&error), started.elapsed())
    })?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| ProcessFailure::without_child(ProcessFailureKind::Io, started.elapsed()))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| ProcessFailure::without_child(ProcessFailureKind::Io, started.elapsed()))?;
    let output_exceeded = Arc::new(AtomicBool::new(false));
    let stdout_reader = capture_stream(stdout, spec.stdout_limit, Arc::clone(&output_exceeded));
    let stderr_reader = capture_stream(stderr, spec.stderr_limit, Arc::clone(&output_exceeded));

    let mut terminal_failure = None;
    let status = loop {
        if output_exceeded.load(Ordering::Acquire) {
            terminal_failure = Some(ProcessFailureKind::OutputLimitExceeded);
            terminate_and_reap(&mut child);
            break child.try_wait().ok().flatten();
        }
        if started.elapsed() >= spec.timeout {
            terminal_failure = Some(ProcessFailureKind::TimedOut);
            terminate_and_reap(&mut child);
            break child.try_wait().ok().flatten();
        }
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) => thread::sleep(POLL_INTERVAL),
            Err(_) => {
                terminal_failure = Some(ProcessFailureKind::Io);
                terminate_and_reap(&mut child);
                break None;
            }
        }
    };

    let stdout = join_capture(stdout_reader);
    let stderr = join_capture(stderr_reader);
    let elapsed = started.elapsed();
    let (stdout, stderr) = match (stdout, stderr) {
        (Ok(stdout), Ok(stderr)) => (stdout, stderr),
        _ => {
            return Err(ProcessFailure::without_child(
                ProcessFailureKind::Io,
                elapsed,
            ));
        }
    };
    let output_limit_exceeded = stdout.exceeded || stderr.exceeded;
    if terminal_failure.is_none() && output_limit_exceeded {
        terminal_failure = Some(ProcessFailureKind::OutputLimitExceeded);
    }
    if let Some(kind) = terminal_failure {
        return Err(ProcessFailure {
            kind,
            exit_code: status.and_then(|value| value.code()),
            stdout: stdout.bytes,
            stderr: stderr.bytes,
            elapsed,
        });
    }
    let status = status
        .ok_or_else(|| ProcessFailure::without_child(ProcessFailureKind::Io, started.elapsed()))?;
    if !status.success() {
        return Err(ProcessFailure {
            kind: ProcessFailureKind::NonZeroExit,
            exit_code: status.code(),
            stdout: stdout.bytes,
            stderr: stderr.bytes,
            elapsed,
        });
    }
    Ok(ProcessOutput {
        status,
        stdout: stdout.bytes,
        stderr: stderr.bytes,
        elapsed,
    })
}

fn classify_spawn_error(error: &io::Error) -> ProcessFailureKind {
    match (error.kind(), error.raw_os_error()) {
        (_, Some(4551 | 1260)) => ProcessFailureKind::EnvironmentBlocked,
        (io::ErrorKind::NotFound, _) => ProcessFailureKind::MissingExecutable,
        _ => ProcessFailureKind::CouldNotStart,
    }
}

fn capture_stream<R>(
    mut reader: R,
    limit: usize,
    output_exceeded: Arc<AtomicBool>,
) -> thread::JoinHandle<io::Result<CapturedStream>>
where
    R: Read + Send + 'static,
{
    thread::spawn(move || {
        let mut retained = Vec::with_capacity(limit.min(64 * 1024));
        let mut buffer = [0u8; READ_BUFFER_BYTES];
        let mut exceeded = false;
        loop {
            let read = reader.read(&mut buffer)?;
            if read == 0 {
                break;
            }
            let remaining = limit.saturating_sub(retained.len());
            let keep = remaining.min(read);
            retained.extend_from_slice(&buffer[..keep]);
            if keep < read {
                exceeded = true;
                output_exceeded.store(true, Ordering::Release);
            }
        }
        Ok(CapturedStream {
            bytes: retained,
            exceeded,
        })
    })
}

fn join_capture(
    handle: thread::JoinHandle<io::Result<CapturedStream>>,
) -> io::Result<CapturedStream> {
    handle
        .join()
        .map_err(|_| io::Error::other("subprocess output reader panicked"))?
}

fn terminate_and_reap(child: &mut std::process::Child) {
    let _ = child.kill();
    let _ = child.wait();
}

#[cfg(test)]
mod tests;
