//! Producer socket for the am fleet monitor.
//!
//! While cm runs agents it listens on its own Unix socket
//! (`<dir>/cm-<pid>.sock`) and streams newline-delimited JSON frames to every
//! connected client: a copy of stdout/stderr, typed orchestration events, and
//! a replay of current state and recent output on connect.
//!
//! cm never connects anywhere and never blocks on a client. When the
//! integration is off (no socket directory resolved, or disabled in config),
//! every function in this module is a no-op: none of them return a `Result`
//! or panic, so callers never need to handle failure.

use std::collections::VecDeque;
use std::fs::DirBuilder;
use std::io::{self, Write};
use std::os::unix::fs::DirBuilderExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use std::time::Duration;

use log::warn;
use serde::Serialize;

use crate::state::Verdict;

/// Number of most-recent output lines kept for replay to newly connected clients.
const OUTPUT_BACKLOG_LINES: usize = 10000;
/// Maximum time a write to a client stream may take before that client is dropped.
const WRITE_TIMEOUT: Duration = Duration::from_millis(100);
/// Value of the `source` field on every frame cm emits.
const SOURCE: &str = "cm";

/// The global producer, set once by [`start`]. Absent means the integration is off.
static AM: OnceLock<Producer> = OnceLock::new();

/// Which output stream a line came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Stream {
    Stdout,
    Stderr,
}

/// Kind of orchestration event, sent in `event` frames.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    Started,
    Progress,
    Completed,
    Deferred,
    Failed,
    NeedsAttention,
    Resolved,
}

/// Structured fields attached to an event, only the ones known at the call site.
#[derive(Debug, Clone, Default, Serialize)]
pub struct EventFields {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phase: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub step: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cycle: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verdict: Option<Verdict>,
}

impl EventFields {
    /// True when no field is set, meaning the frame should omit `fields` entirely.
    fn is_empty(&self) -> bool {
        self.phase.is_none() && self.step.is_none() && self.cycle.is_none() && self.verdict.is_none()
    }
}

/// A single NDJSON frame sent to clients. Built only through this enum and `serde_json`.
#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum Frame {
    Event {
        kind: EventKind,
        source: &'static str,
        pid: u32,
        message: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        task: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pct: Option<f64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        level: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        fields: Option<EventFields>,
    },
    Output {
        source: &'static str,
        pid: u32,
        seq: u64,
        stream: Stream,
        text: String,
    },
    State {
        source: &'static str,
        pid: u32,
        #[serde(skip_serializing_if = "Option::is_none")]
        task: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        attention: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        attention_message: Option<String>,
    },
}

/// One backlogged output line, kept for replay to newly connected clients.
struct BacklogLine {
    seq: u64,
    stream: Stream,
    text: String,
}

/// Shared, mutex-guarded state of a [`Producer`].
struct Inner {
    path: PathBuf,
    clients: Vec<UnixStream>,
    next_seq: u64,
    backlog: VecDeque<BacklogLine>,
    task: Option<String>,
    attention: Option<String>,
    partial_stdout: Vec<u8>,
    partial_stderr: Vec<u8>,
}

/// Producer side of the am socket: accepts client connections and broadcasts frames.
pub struct Producer {
    inner: Arc<Mutex<Inner>>,
}

impl Producer {
    /// Create the socket directory (mode 0700) if missing, remove a stale file at the
    /// socket path, bind the listener, and spawn the accept thread.
    fn bind(dir: &Path) -> io::Result<Self> {
        let mut builder = DirBuilder::new();
        builder.recursive(true);
        builder.mode(0o700);
        builder.create(dir)?;

        let path = dir.join(format!("cm-{}.sock", std::process::id()));
        if path.exists() {
            std::fs::remove_file(&path)?;
        }

        let listener = UnixListener::bind(&path)?;

        let inner = Arc::new(Mutex::new(Inner {
            path,
            clients: Vec::new(),
            next_seq: 1,
            backlog: VecDeque::new(),
            task: None,
            attention: None,
            partial_stdout: Vec::new(),
            partial_stderr: Vec::new(),
        }));

        let accept_inner = Arc::clone(&inner);
        thread::spawn(move || accept_loop(listener, accept_inner));

        Ok(Self { inner })
    }

    /// Remove the socket file. Connected clients see EOF when the process exits.
    fn shutdown(&self) {
        if let Ok(guard) = self.inner.lock() {
            let _ = std::fs::remove_file(&guard.path);
        }
    }

    /// Record the event's effect on tracked state, then broadcast it to every client.
    fn send_event(&self, kind: EventKind, task: Option<&str>, message: &str, fields: EventFields) {
        let mut guard = match self.inner.lock() {
            Ok(g) => g,
            Err(_) => return,
        };

        match kind {
            EventKind::Started | EventKind::Progress => {
                if let Some(t) = task {
                    guard.task = Some(t.to_string());
                }
            }
            EventKind::Completed | EventKind::Deferred | EventKind::Failed => {
                guard.task = None;
            }
            EventKind::NeedsAttention => {
                guard.attention = Some(message.to_string());
            }
            EventKind::Resolved => {
                guard.attention = None;
            }
        }

        let frame = Frame::Event {
            kind,
            source: SOURCE,
            pid: std::process::id(),
            message: message.to_string(),
            task: task.map(String::from),
            pct: None,
            level: None,
            fields: if fields.is_empty() { None } else { Some(fields) },
        };
        broadcast(&mut guard, &frame);
    }

    /// Append `bytes` to the stream's partial buffer, emit one `output` frame per
    /// complete line, and keep the remainder for the next call.
    fn send_output(&self, stream: Stream, bytes: &[u8]) {
        let lines = {
            let mut guard = match self.inner.lock() {
                Ok(g) => g,
                Err(_) => return,
            };
            let partial = partial_buffer(&mut guard, stream);
            append_and_split(partial, bytes)
        };
        self.emit_lines(stream, lines);
    }

    /// Same as [`Self::send_output`], then flush the remaining partial as its own line.
    fn send_output_line(&self, stream: Stream, bytes: &[u8]) {
        let lines = {
            let mut guard = match self.inner.lock() {
                Ok(g) => g,
                Err(_) => return,
            };
            let partial = partial_buffer(&mut guard, stream);
            let mut lines = append_and_split(partial, bytes);
            if !partial.is_empty() {
                lines.push(clean_line(partial));
                partial.clear();
            }
            lines
        };
        self.emit_lines(stream, lines);
    }

    /// Assign sequence numbers, push each line to the backlog, and broadcast it.
    fn emit_lines(&self, stream: Stream, lines: Vec<String>) {
        if lines.is_empty() {
            return;
        }
        let mut guard = match self.inner.lock() {
            Ok(g) => g,
            Err(_) => return,
        };
        for text in lines {
            let seq = guard.next_seq;
            guard.next_seq += 1;
            guard.backlog.push_back(BacklogLine { seq, stream, text: text.clone() });
            if guard.backlog.len() > OUTPUT_BACKLOG_LINES {
                guard.backlog.pop_front();
            }
            let frame = Frame::Output { source: SOURCE, pid: std::process::id(), seq, stream, text };
            broadcast(&mut guard, &frame);
        }
    }
}

/// Borrow the partial-line buffer for the given stream.
fn partial_buffer(guard: &mut Inner, stream: Stream) -> &mut Vec<u8> {
    match stream {
        Stream::Stdout => &mut guard.partial_stdout,
        Stream::Stderr => &mut guard.partial_stderr,
    }
}

/// Append `bytes` to `partial`, split out every complete line (decoded lossily,
/// `\r` removed, trailing whitespace trimmed), and leave the remainder in `partial`.
fn append_and_split(partial: &mut Vec<u8>, bytes: &[u8]) -> Vec<String> {
    partial.extend_from_slice(bytes);
    let mut lines = Vec::new();
    while let Some(pos) = partial.iter().position(|&b| b == b'\n') {
        let line: Vec<u8> = partial.drain(..=pos).collect();
        lines.push(clean_line(&line[..line.len() - 1]));
    }
    lines
}

/// Decode bytes lossily, strip `\r`, and trim trailing whitespace.
fn clean_line(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).replace('\r', "").trim_end().to_string()
}

/// Serialize a frame as one NDJSON line and write it to `stream`.
fn write_frame(stream: &mut UnixStream, frame: &Frame) -> bool {
    let mut json = match serde_json::to_vec(frame) {
        Ok(v) => v,
        Err(_) => return false,
    };
    json.push(b'\n');
    stream.write_all(&json).is_ok()
}

/// Write `frame` to every client, dropping any whose write fails or times out.
fn broadcast(guard: &mut Inner, frame: &Frame) {
    guard.clients.retain_mut(|client| write_frame(client, frame));
}

/// Accept loop: for each new connection, replay state and backlog, then keep it as a
/// live client. Runs for the lifetime of the process on its own thread.
fn accept_loop(listener: UnixListener, inner: Arc<Mutex<Inner>>) {
    for incoming in listener.incoming() {
        let stream = match incoming {
            Ok(s) => s,
            Err(_) => continue,
        };
        if stream.set_write_timeout(Some(WRITE_TIMEOUT)).is_err() {
            continue;
        }
        let mut guard = match inner.lock() {
            Ok(g) => g,
            Err(_) => return,
        };
        accept_client(&mut guard, stream);
    }
}

/// Write the `state` frame, then the output backlog in order, to a new client.
/// On success the stream joins the live client list; on any write failure it is dropped.
fn accept_client(guard: &mut Inner, mut stream: UnixStream) {
    let state_frame = Frame::State {
        source: SOURCE,
        pid: std::process::id(),
        task: guard.task.clone(),
        attention: guard.attention.as_ref().map(|_| true),
        attention_message: guard.attention.clone(),
    };
    if !write_frame(&mut stream, &state_frame) {
        return;
    }

    for line in &guard.backlog {
        let frame = Frame::Output {
            source: SOURCE,
            pid: std::process::id(),
            seq: line.seq,
            stream: line.stream,
            text: line.text.clone(),
        };
        if !write_frame(&mut stream, &frame) {
            return;
        }
    }

    guard.clients.push(stream);
}

/// Start the am socket in `dir`, if not already started. A no-op when `dir` is `None`
/// or the producer was already started. Logs a warning and leaves the integration off
/// if binding fails.
pub fn start(dir: Option<&Path>) {
    if AM.get().is_some() {
        return;
    }
    let dir = match dir {
        Some(d) => d,
        None => return,
    };
    match Producer::bind(dir) {
        Ok(producer) => {
            let _ = AM.set(producer);
        }
        Err(e) => warn!("am: failed to bind socket in {dir:?}: {e}, integration off"),
    }
}

/// Send an `event` frame. No-op when the integration is off.
pub fn send_event(kind: EventKind, task: Option<&str>, message: &str, fields: EventFields) {
    if let Some(p) = AM.get() {
        p.send_event(kind, task, message, fields);
    }
}

/// Send `bytes` as output, splitting complete lines into `output` frames and keeping
/// any trailing partial line for the next call. No-op when the integration is off.
pub fn send_output(stream: Stream, bytes: &[u8]) {
    if let Some(p) = AM.get() {
        p.send_output(stream, bytes);
    }
}

/// Same as [`send_output`], then flushes the remaining partial line as its own frame.
/// No-op when the integration is off.
pub fn send_output_line(stream: Stream, bytes: &[u8]) {
    if let Some(p) = AM.get() {
        p.send_output_line(stream, bytes);
    }
}

/// Remove the socket file, if the producer was started. No-op otherwise.
pub fn shutdown() {
    if let Some(p) = AM.get() {
        p.shutdown();
    }
}

#[cfg(test)]
impl Producer {
    fn socket_path(&self) -> PathBuf {
        self.inner.lock().map(|g| g.path.clone()).unwrap_or_default()
    }

    fn client_count(&self) -> usize {
        self.inner.lock().map(|g| g.clients.len()).unwrap_or(0)
    }

    fn backlog_snapshot(&self) -> Vec<(u64, Stream, String)> {
        self.inner
            .lock()
            .map(|g| g.backlog.iter().map(|l| (l.seq, l.stream, l.text.clone())).collect())
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use std::io::{BufRead, BufReader};
    use std::os::unix::net::UnixStream;
    use std::time::Duration;

    use tempfile::TempDir;

    use crate::am::{EventFields, EventKind, Frame, Producer, Stream, SOURCE};
    use crate::state::Verdict;

    #[test]
    fn frame_event_serializes_with_fields() {
        let frame = Frame::Event {
            kind: EventKind::Progress,
            source: SOURCE,
            pid: 1234,
            message: "Phase review cycle 2/5 for phase-41".to_string(),
            task: Some("phase-41 PHASE_REVIEW".to_string()),
            pct: None,
            level: None,
            fields: Some(EventFields {
                phase: Some("phase-41".to_string()),
                step: Some("PHASE_REVIEW".to_string()),
                cycle: Some(2),
                verdict: None,
            }),
        };
        let json = serde_json::to_string(&frame).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(value["type"], "event");
        assert_eq!(value["kind"], "progress");
        assert_eq!(value["source"], "cm");
        assert_eq!(value["pid"], 1234);
        assert_eq!(value["task"], "phase-41 PHASE_REVIEW");
        assert_eq!(value["message"], "Phase review cycle 2/5 for phase-41");
        assert_eq!(value["fields"]["phase"], "phase-41");
        assert_eq!(value["fields"]["step"], "PHASE_REVIEW");
        assert_eq!(value["fields"]["cycle"], 2);
        assert!(value.get("pct").is_none());
        assert!(value.get("level").is_none());
        assert!(value["fields"].get("verdict").is_none());
    }

    #[test]
    fn frame_event_omits_absent_fields_and_task() {
        let frame = Frame::Event {
            kind: EventKind::NeedsAttention,
            source: SOURCE,
            pid: 42,
            message: "Select tasks: [s]ingle / [a]ll / [p]hase / [q]uit".to_string(),
            task: None,
            pct: None,
            level: None,
            fields: None,
        };
        let json = serde_json::to_string(&frame).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(value["kind"], "needs_attention");
        assert!(value.get("task").is_none());
        assert!(value.get("fields").is_none());
    }

    #[test]
    fn frame_output_serializes() {
        let frame = Frame::Output {
            source: SOURCE,
            pid: 1234,
            seq: 17,
            stream: Stream::Stderr,
            text: "[2026-09-17T10:00:00Z CM] Selected phase: phase-41".to_string(),
        };
        let json = serde_json::to_string(&frame).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(value["type"], "output");
        assert_eq!(value["seq"], 17);
        assert_eq!(value["stream"], "stderr");
        assert_eq!(value["text"], "[2026-09-17T10:00:00Z CM] Selected phase: phase-41");
    }

    #[test]
    fn frame_state_serializes_with_attention() {
        let frame = Frame::State {
            source: SOURCE,
            pid: 1234,
            task: Some("phase-41 PHASE_REVIEW".to_string()),
            attention: Some(true),
            attention_message: Some(
                "Task phase-41 exhausted 5 review cycles. Retry more cycles?".to_string(),
            ),
        };
        let json = serde_json::to_string(&frame).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(value["type"], "state");
        assert_eq!(value["attention"], true);
        assert_eq!(
            value["attention_message"],
            "Task phase-41 exhausted 5 review cycles. Retry more cycles?"
        );
    }

    #[test]
    fn frame_state_omits_absent_fields() {
        let frame = Frame::State {
            source: SOURCE,
            pid: 1234,
            task: None,
            attention: None,
            attention_message: None,
        };
        let json = serde_json::to_string(&frame).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(value.get("task").is_none());
        assert!(value.get("attention").is_none());
        assert!(value.get("attention_message").is_none());
    }

    #[test]
    fn event_fields_verdict_spelling() {
        let approved = EventFields { verdict: Some(Verdict::Approved), ..Default::default() };
        let json = serde_json::to_string(&approved).unwrap();
        assert!(json.contains("\"verdict\":\"approved\""));

        let needs_fixes = EventFields { verdict: Some(Verdict::NeedsFixes), ..Default::default() };
        let json = serde_json::to_string(&needs_fixes).unwrap();
        assert!(json.contains("\"verdict\":\"needs_fixes\""));
    }

    #[test]
    fn first_output_line_has_seq_one() {
        let tmp = TempDir::new().unwrap();
        let producer = Producer::bind(tmp.path()).unwrap();
        producer.send_output(Stream::Stdout, b"hello\n");
        let backlog = producer.backlog_snapshot();
        assert_eq!(backlog[0].0, 1);
    }

    #[test]
    fn output_splits_across_calls() {
        let tmp = TempDir::new().unwrap();
        let producer = Producer::bind(tmp.path()).unwrap();
        producer.send_output(Stream::Stdout, b"a\nb");
        producer.send_output(Stream::Stdout, b"c\n");
        let lines: Vec<String> =
            producer.backlog_snapshot().into_iter().map(|(_, _, t)| t).collect();
        assert_eq!(lines, vec!["a".to_string(), "bc".to_string()]);
    }

    #[test]
    fn output_strips_cr_and_trailing_whitespace() {
        let tmp = TempDir::new().unwrap();
        let producer = Producer::bind(tmp.path()).unwrap();
        producer.send_output(Stream::Stdout, b"hello  \r\n");
        let lines: Vec<String> =
            producer.backlog_snapshot().into_iter().map(|(_, _, t)| t).collect();
        assert_eq!(lines, vec!["hello".to_string()]);
    }

    #[test]
    fn output_multibyte_utf8_split_across_calls() {
        let tmp = TempDir::new().unwrap();
        let producer = Producer::bind(tmp.path()).unwrap();
        let euro = "\u{20ac}";
        let bytes = euro.as_bytes();
        producer.send_output(Stream::Stdout, &bytes[..1]);
        producer.send_output(Stream::Stdout, &bytes[1..]);
        producer.send_output(Stream::Stdout, b"\n");
        let lines: Vec<String> =
            producer.backlog_snapshot().into_iter().map(|(_, _, t)| t).collect();
        assert_eq!(lines, vec![euro.to_string()]);
    }

    #[test]
    fn send_output_line_flushes_pending_partial() {
        let tmp = TempDir::new().unwrap();
        let producer = Producer::bind(tmp.path()).unwrap();
        producer.send_output(Stream::Stdout, b"a");
        producer.send_output_line(Stream::Stdout, b"x");
        let lines: Vec<String> =
            producer.backlog_snapshot().into_iter().map(|(_, _, t)| t).collect();
        assert_eq!(lines, vec!["ax".to_string()]);
    }

    #[test]
    fn send_output_line_newline_then_prompt() {
        let tmp = TempDir::new().unwrap();
        let producer = Producer::bind(tmp.path()).unwrap();
        producer.send_output_line(Stream::Stdout, b"\nq: ");
        let lines: Vec<String> =
            producer.backlog_snapshot().into_iter().map(|(_, _, t)| t).collect();
        assert_eq!(lines, vec!["".to_string(), "q:".to_string()]);
    }

    #[test]
    fn backlog_replay_keeps_seq_and_window() {
        let tmp = TempDir::new().unwrap();
        let producer = Producer::bind(tmp.path()).unwrap();
        let path = producer.socket_path();

        for i in 0..10100u32 {
            producer.send_output(Stream::Stdout, format!("line-{i}\n").as_bytes());
        }

        let stream = UnixStream::connect(&path).unwrap();
        stream.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
        let mut reader = BufReader::new(stream);

        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        let state: serde_json::Value = serde_json::from_str(line.trim_end()).unwrap();
        assert_eq!(state["type"], "state");

        for expected_seq in 101u64..=10100u64 {
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            let frame: serde_json::Value = serde_json::from_str(line.trim_end()).unwrap();
            assert_eq!(frame["type"], "output");
            assert_eq!(frame["seq"], expected_seq);
        }

        producer.send_output(Stream::Stdout, b"live\n");
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        let frame: serde_json::Value = serde_json::from_str(line.trim_end()).unwrap();
        assert_eq!(frame["seq"], 10101);
        assert_eq!(frame["text"], "live");
    }

    #[test]
    fn state_tracks_task_and_attention() {
        let tmp = TempDir::new().unwrap();
        let producer = Producer::bind(tmp.path()).unwrap();
        producer.send_event(
            EventKind::Progress,
            Some("phase-1 PHASE_IMPLEM"),
            "Spawning IMPLEM agent",
            EventFields::default(),
        );
        producer.send_event(
            EventKind::NeedsAttention,
            None,
            "Retry more cycles?",
            EventFields::default(),
        );

        let path = producer.socket_path();
        let stream = UnixStream::connect(&path).unwrap();
        stream.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
        let mut reader = BufReader::new(stream);
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        let state: serde_json::Value = serde_json::from_str(line.trim_end()).unwrap();
        assert_eq!(state["task"], "phase-1 PHASE_IMPLEM");
        assert_eq!(state["attention"], true);
        assert_eq!(state["attention_message"], "Retry more cycles?");
    }

    #[test]
    fn state_clears_attention_after_resolved() {
        let tmp = TempDir::new().unwrap();
        let producer = Producer::bind(tmp.path()).unwrap();
        producer.send_event(EventKind::NeedsAttention, None, "Select tasks", EventFields::default());
        producer.send_event(EventKind::Resolved, None, "", EventFields::default());

        let path = producer.socket_path();
        let stream = UnixStream::connect(&path).unwrap();
        stream.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
        let mut reader = BufReader::new(stream);
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        let state: serde_json::Value = serde_json::from_str(line.trim_end()).unwrap();
        assert!(state.get("attention").is_none());
    }

    #[test]
    fn state_clears_task_after_completed() {
        let tmp = TempDir::new().unwrap();
        let producer = Producer::bind(tmp.path()).unwrap();
        producer.send_event(
            EventKind::Progress,
            Some("phase-1"),
            "Executing phase: phase-1",
            EventFields::default(),
        );
        producer.send_event(EventKind::Completed, Some("phase-1"), "Phase complete", EventFields::default());

        let path = producer.socket_path();
        let stream = UnixStream::connect(&path).unwrap();
        stream.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
        let mut reader = BufReader::new(stream);
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        let state: serde_json::Value = serde_json::from_str(line.trim_end()).unwrap();
        assert!(state.get("task").is_none());
    }

    #[test]
    fn run_review_state_never_carries_task() {
        let tmp = TempDir::new().unwrap();
        let producer = Producer::bind(tmp.path()).unwrap();
        producer.send_event(
            EventKind::Progress,
            Some("run-review RUN_REVIEW"),
            "Spawning run review agent",
            EventFields::default(),
        );
        producer.send_event(
            EventKind::Failed,
            Some("run-review RUN_REVIEW"),
            "Run review: issues found",
            EventFields::default(),
        );

        let path = producer.socket_path();
        let stream = UnixStream::connect(&path).unwrap();
        stream.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
        let mut reader = BufReader::new(stream);
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        let state: serde_json::Value = serde_json::from_str(line.trim_end()).unwrap();
        assert!(state.get("task").is_none());
    }

    #[test]
    fn send_with_no_client_does_not_panic() {
        let tmp = TempDir::new().unwrap();
        let producer = Producer::bind(tmp.path()).unwrap();
        producer.send_event(EventKind::Started, Some("phase-1"), "Executing phase: phase-1", EventFields::default());
        producer.send_output(Stream::Stdout, b"hello\n");
        assert_eq!(producer.client_count(), 0);
        assert_eq!(producer.backlog_snapshot().len(), 1);
    }

    #[test]
    fn dropped_client_is_removed_on_next_send() {
        let tmp = TempDir::new().unwrap();
        let producer = Producer::bind(tmp.path()).unwrap();
        let path = producer.socket_path();

        let stream1 = UnixStream::connect(&path).unwrap();
        drop(stream1);
        std::thread::sleep(Duration::from_millis(100));
        assert_eq!(producer.client_count(), 1);

        let stream2 = UnixStream::connect(&path).unwrap();
        stream2.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
        let mut reader2 = BufReader::new(stream2);
        let mut line = String::new();
        reader2.read_line(&mut line).unwrap();
        std::thread::sleep(Duration::from_millis(100));
        assert_eq!(producer.client_count(), 2);

        producer.send_event(EventKind::Started, Some("phase-1"), "Executing phase: phase-1", EventFields::default());

        let mut line2 = String::new();
        reader2.read_line(&mut line2).unwrap();
        let value: serde_json::Value = serde_json::from_str(line2.trim_end()).unwrap();
        assert_eq!(value["type"], "event");

        assert_eq!(producer.client_count(), 1);
    }

    #[test]
    fn stuck_client_is_dropped_after_write_timeout() {
        let tmp = TempDir::new().unwrap();
        let producer = Producer::bind(tmp.path()).unwrap();
        let path = producer.socket_path();

        let stream = UnixStream::connect(&path).unwrap();
        std::thread::sleep(Duration::from_millis(100));
        assert_eq!(producer.client_count(), 1);

        let big_line = "x".repeat(1_000_000);
        let start = std::time::Instant::now();
        for _ in 0..50 {
            producer.send_output(Stream::Stdout, format!("{big_line}\n").as_bytes());
        }
        let elapsed = start.elapsed();

        assert!(elapsed < Duration::from_secs(10));
        assert_eq!(producer.client_count(), 0);
        drop(stream);
    }

    #[test]
    fn socket_file_created_with_correct_path_and_dir_mode() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("am");
        let producer = Producer::bind(&dir).unwrap();
        let expected_path = dir.join(format!("cm-{}.sock", std::process::id()));
        assert_eq!(producer.socket_path(), expected_path);
        assert!(expected_path.exists());

        let meta = std::fs::metadata(&dir).unwrap();
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(meta.permissions().mode() & 0o777, 0o700);
    }

    #[test]
    fn socket_bind_replaces_stale_file() {
        let tmp = TempDir::new().unwrap();
        let stale_path = tmp.path().join(format!("cm-{}.sock", std::process::id()));
        std::fs::write(&stale_path, b"not a socket").unwrap();

        let producer = Producer::bind(tmp.path()).unwrap();
        assert_eq!(producer.socket_path(), stale_path);
        assert!(stale_path.exists());
    }

    #[test]
    fn shutdown_removes_socket_file() {
        let tmp = TempDir::new().unwrap();
        let producer = Producer::bind(tmp.path()).unwrap();
        let path = producer.socket_path();
        assert!(path.exists());
        producer.shutdown();
        assert!(!path.exists());
    }
}
