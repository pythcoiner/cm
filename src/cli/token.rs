//! `cm token` subcommand.
//!
//! Walks Claude Code's per-project session JSONL files and prints token
//! consumption with an estimated USD cost. Pricing rates come from
//! `.cm/config.toml` (with hardcoded defaults as a fallback); the
//! `/update-pricing` slash command refreshes the config without a rebuild.

use std::borrow::Cow;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use chrono::{DateTime, Datelike, Utc};
use log::{debug, warn};
use serde::Deserialize;
use thiserror::Error;

use crate::config::{ConfigFile, PriceTable};

/// Hardcoded fallback rates (USD per million tokens).
///
/// Ported from `ccusage-statusline-rs/src/pricing.rs` fallback table; kept
/// here so `cm token` works offline and out-of-the-box. Override via
/// `.cm/config.toml` (`/update-pricing` keeps the file fresh).
const DEFAULT_RATES: &[(&str, [f64; 4])] = &[
    // family, [input, output, cache_write, cache_read] $/1M
    ("opus", [15.0, 75.0, 18.75, 1.50]),
    ("sonnet", [3.0, 15.0, 3.75, 0.30]),
    ("haiku", [0.8, 4.0, 1.00, 0.08]),
];

/// Family keys checked in order for substring fallback. Matches
/// `DEFAULT_RATES` so behaviour is consistent.
const FAMILY_KEYS: &[&str] = &["opus", "sonnet", "haiku"];

/// Errors produced by `cm token`.
#[derive(Debug, Error)]
pub enum TokenError {
    /// An I/O error occurred while reading session files.
    #[error("io error: {0}")]
    IoError(#[from] std::io::Error),

    /// No `~/.claude/projects/` directory could be located.
    #[error("no Claude Code projects directory found (looked at $CLAUDE_CONFIG_DIR/projects, ~/.claude/projects, ~/.config/claude/projects)")]
    NoProjectsDir,

    /// Could not resolve the user's home directory.
    #[error("could not resolve home directory")]
    HomeNotFound,

    /// Could not resolve the current working directory.
    #[error("could not resolve current working directory: {0}")]
    CwdNotFound(std::io::Error),
}

/// Time-bucket aggregation modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BucketMode {
    Daily,
    Weekly,
    Monthly,
}

/// Options controlling `execute_token`.
#[derive(Debug, Clone, Copy)]
pub struct TokenOpts {
    /// If `true`, aggregate across every project in the Claude Code
    /// projects directory; otherwise scope to the current cwd's project.
    pub global: bool,
    /// Optional time-bucket grouping. When set, per-row breakdowns swap
    /// from per-model to per-bucket.
    pub bucket: Option<BucketMode>,
}

/// Accumulated token counts.
#[derive(Debug, Default, Clone, Copy)]
struct UsageRow {
    input: u64,
    output: u64,
    cache_write: u64,
    cache_read: u64,
}

impl UsageRow {
    fn add(&mut self, usage: &Usage) {
        self.input = self.input.saturating_add(usage.input_tokens);
        self.output = self.output.saturating_add(usage.output_tokens);
        self.cache_write = self
            .cache_write
            .saturating_add(usage.cache_creation_input_tokens);
        self.cache_read = self
            .cache_read
            .saturating_add(usage.cache_read_input_tokens);
    }
}

#[derive(Debug, Deserialize)]
struct Entry {
    #[serde(default)]
    message: Option<Message>,
    #[serde(default, rename = "requestId")]
    request_id: Option<String>,
    #[serde(default)]
    timestamp: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Message {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    usage: Option<Usage>,
}

#[derive(Debug, Deserialize, Default)]
struct Usage {
    #[serde(default)]
    input_tokens: u64,
    #[serde(default)]
    output_tokens: u64,
    #[serde(default)]
    cache_creation_input_tokens: u64,
    #[serde(default)]
    cache_read_input_tokens: u64,
}

/// Top-level entry point for the subcommand.
pub fn execute_token(opts: TokenOpts) -> Result<(), TokenError> {
    let projects = projects_dir()?;
    let config = ConfigFile::load_default().ok().flatten().unwrap_or_default();
    let overrides = config.pricing.unwrap_or_default();

    // Determine scope and label.
    let (paths, label) = if opts.global {
        (collect_global_jsonl(&projects)?, "all projects".to_string())
    } else {
        let cwd = std::env::current_dir().map_err(TokenError::CwdNotFound)?;
        let slug = project_slug(&cwd);
        let project_dir = projects.join(&slug);
        let paths = collect_project_jsonl(&project_dir)?;
        (paths, cwd.display().to_string())
    };

    // Aggregate.
    let mut seen: HashSet<String> = HashSet::new();
    let mut by_model: BTreeMap<String, UsageRow> = BTreeMap::new();
    let mut by_bucket: BTreeMap<String, UsageRow> = BTreeMap::new();
    let mut bucket_models: BTreeMap<String, BTreeMap<String, UsageRow>> = BTreeMap::new();

    for path in &paths {
        if let Err(err) = parse_jsonl(
            path,
            &mut seen,
            &mut by_model,
            opts.bucket.map(|m| (m, &mut by_bucket, &mut bucket_models)),
        ) {
            warn!("skipping {}: {err}", path.display());
        }
    }

    // Render.
    if let Some(mode) = opts.bucket {
        print_bucket_report(&label, &by_bucket, &bucket_models, &overrides, mode);
    } else {
        print_flat_report(&label, &by_model, &overrides);
    }

    Ok(())
}

/// Convert a filesystem path into a Claude Code "project slug", matching
/// the directory naming used under `~/.claude/projects/`.
///
/// Empirically: every `/` and `.` is replaced by `-`. So
/// `/home/pyth/cm` → `-home-pyth-cm` and
/// `/home/pyth/.config/nvim` → `-home-pyth--config-nvim`.
fn project_slug(path: &Path) -> String {
    let s = path.to_string_lossy();
    s.chars()
        .map(|c| match c {
            '/' | '.' => '-',
            other => other,
        })
        .collect()
}

/// Resolve the Claude Code `projects/` directory.
fn projects_dir() -> Result<PathBuf, TokenError> {
    if let Some(env) = std::env::var_os("CLAUDE_CONFIG_DIR") {
        let candidate = PathBuf::from(env).join("projects");
        if candidate.is_dir() {
            return Ok(candidate);
        }
    }

    let home = dirs::home_dir().ok_or(TokenError::HomeNotFound)?;

    let claude = home.join(".claude").join("projects");
    if claude.is_dir() {
        return Ok(claude);
    }

    let xdg = home.join(".config").join("claude").join("projects");
    if xdg.is_dir() {
        return Ok(xdg);
    }

    Err(TokenError::NoProjectsDir)
}

fn collect_project_jsonl(dir: &Path) -> Result<Vec<PathBuf>, TokenError> {
    if !dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "jsonl") {
            out.push(path);
        }
    }
    Ok(out)
}

fn collect_global_jsonl(projects: &Path) -> Result<Vec<PathBuf>, TokenError> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(projects)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            out.extend(collect_project_jsonl(&path)?);
        }
    }
    Ok(out)
}

type BucketSink<'a> = (
    BucketMode,
    &'a mut BTreeMap<String, UsageRow>,
    &'a mut BTreeMap<String, BTreeMap<String, UsageRow>>,
);

fn parse_jsonl(
    path: &Path,
    seen: &mut HashSet<String>,
    by_model: &mut BTreeMap<String, UsageRow>,
    bucket_sink: Option<BucketSink<'_>>,
) -> Result<(), TokenError> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let (mode, mut by_bucket, mut bucket_models) = match bucket_sink {
        Some((m, b, bm)) => (Some(m), Some(b), Some(bm)),
        None => (None, None, None),
    };

    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let entry: Entry = match serde_json::from_str(&line) {
            Ok(e) => e,
            Err(err) => {
                debug!("skip malformed line in {}: {err}", path.display());
                continue;
            }
        };

        let Some(msg) = entry.message else { continue };
        let Some(usage) = msg.usage else { continue };
        let Some(model) = msg.model else { continue };

        // Dedup using msg_id:request_id (matches ccusage-statusline-rs).
        if let (Some(id), Some(req)) = (msg.id.as_deref(), entry.request_id.as_deref()) {
            let key = format!("{id}:{req}");
            if !seen.insert(key) {
                continue;
            }
        }

        // Per-model accumulator (always populated; used as grand-total source).
        by_model.entry(model.clone()).or_default().add(&usage);

        // Per-bucket accumulator.
        if let (Some(mode), Some(by_bucket), Some(bucket_models)) =
            (mode, by_bucket.as_deref_mut(), bucket_models.as_deref_mut())
        {
            let label = entry
                .timestamp
                .as_deref()
                .and_then(|ts| bucket_label(ts, mode));
            if let Some(label) = label {
                by_bucket.entry(label.clone()).or_default().add(&usage);
                bucket_models
                    .entry(label)
                    .or_default()
                    .entry(model)
                    .or_default()
                    .add(&usage);
            }
        }
    }

    Ok(())
}

fn bucket_label(ts: &str, mode: BucketMode) -> Option<String> {
    let dt = DateTime::parse_from_rfc3339(ts).ok()?.with_timezone(&Utc);
    Some(match mode {
        BucketMode::Daily => dt.format("%Y-%m-%d").to_string(),
        BucketMode::Weekly => {
            let iso = dt.iso_week();
            format!("{}-W{:02}", iso.year(), iso.week())
        }
        BucketMode::Monthly => dt.format("%Y-%m").to_string(),
    })
}

/// Look up rates for a model. Order: exact override → family-substring
/// override → hardcoded family default → `None`.
fn lookup_rates<'a>(
    model: &str,
    overrides: &'a HashMap<String, PriceTable>,
) -> Option<Cow<'a, PriceTable>> {
    if let Some(t) = overrides.get(model) {
        return Some(Cow::Borrowed(t));
    }
    let lower = model.to_lowercase();
    for fam in FAMILY_KEYS {
        if lower.contains(fam) {
            if let Some(t) = overrides.get(*fam) {
                return Some(Cow::Borrowed(t));
            }
        }
    }
    for (fam, rates) in DEFAULT_RATES {
        if lower.contains(fam) {
            return Some(Cow::Owned(PriceTable {
                input: rates[0],
                output: rates[1],
                cache_write: rates[2],
                cache_read: rates[3],
            }));
        }
    }
    None
}

fn cost(rates: &PriceTable, row: &UsageRow) -> f64 {
    let m = 1_000_000.0;
    (row.input as f64) * rates.input / m
        + (row.output as f64) * rates.output / m
        + (row.cache_write as f64) * rates.cache_write / m
        + (row.cache_read as f64) * rates.cache_read / m
}

fn format_tokens(n: u64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.0}K", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}

fn print_flat_report(
    label: &str,
    by_model: &BTreeMap<String, UsageRow>,
    overrides: &HashMap<String, PriceTable>,
) {
    println!("Claude Code usage — {label}");
    if by_model.is_empty() {
        println!("  (no recorded usage)");
        return;
    }

    let model_w = by_model.keys().map(|s| s.len()).max().unwrap_or(0).max(8);
    let mut total = 0.0f64;
    let mut unpriced: Vec<&str> = Vec::new();

    for (model, row) in by_model {
        let usd = lookup_rates(model, overrides).map(|r| cost(&r, row));
        match usd {
            Some(c) => total += c,
            None => unpriced.push(model.as_str()),
        }
        println!(
            "  {:<model_w$}  in: {:<7} out: {:<7} cache_w: {:<7} cache_r: {:<7}  {}",
            model,
            format_tokens(row.input),
            format_tokens(row.output),
            format_tokens(row.cache_write),
            format_tokens(row.cache_read),
            usd.map(|c| format!("${c:.2}"))
                .unwrap_or_else(|| "(no rate)".to_string()),
            model_w = model_w,
        );
    }

    println!("  {:─<width$}", "", width = 70 + model_w);
    println!(
        "  {:<model_w$}{:>w$}  ${:.2}",
        "total",
        "",
        total,
        model_w = model_w,
        w = 70 - 6,
    );

    for m in unpriced {
        warn!("no pricing for model `{m}` — counted in tokens, excluded from $ total");
    }
}

fn print_bucket_report(
    label: &str,
    by_bucket: &BTreeMap<String, UsageRow>,
    bucket_models: &BTreeMap<String, BTreeMap<String, UsageRow>>,
    overrides: &HashMap<String, PriceTable>,
    mode: BucketMode,
) {
    let mode_label = match mode {
        BucketMode::Daily => "daily",
        BucketMode::Weekly => "weekly",
        BucketMode::Monthly => "monthly",
    };
    println!("Claude Code usage — {label} ({mode_label})");
    if by_bucket.is_empty() {
        println!("  (no recorded usage)");
        return;
    }

    let bucket_w = by_bucket.keys().map(|s| s.len()).max().unwrap_or(0).max(8);
    let mut total = 0.0f64;

    for (bucket, row) in by_bucket {
        let usd = bucket_models
            .get(bucket)
            .map(|per_model| {
                per_model
                    .iter()
                    .filter_map(|(m, r)| lookup_rates(m, overrides).map(|rates| cost(&rates, r)))
                    .sum::<f64>()
            })
            .unwrap_or(0.0);
        total += usd;
        println!(
            "  {:<bucket_w$}  in: {:<7} out: {:<7} cache_w: {:<7} cache_r: {:<7}  ${:.2}",
            bucket,
            format_tokens(row.input),
            format_tokens(row.output),
            format_tokens(row.cache_write),
            format_tokens(row.cache_read),
            usd,
            bucket_w = bucket_w,
        );
    }

    println!("  {:─<width$}", "", width = 70 + bucket_w);
    println!(
        "  {:<bucket_w$}{:>w$}  ${:.2}",
        "total",
        "",
        total,
        bucket_w = bucket_w,
        w = 70 - 6,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rt(input: f64, output: f64, cw: f64, cr: f64) -> PriceTable {
        PriceTable {
            input,
            output,
            cache_write: cw,
            cache_read: cr,
        }
    }

    #[test]
    fn project_slug_basic() {
        assert_eq!(project_slug(Path::new("/home/pyth/cm")), "-home-pyth-cm");
    }

    #[test]
    fn project_slug_dotfile_segment() {
        assert_eq!(
            project_slug(Path::new("/home/pyth/.config/nvim")),
            "-home-pyth--config-nvim"
        );
    }

    #[test]
    fn lookup_exact_beats_family_beats_default() {
        let mut overrides: HashMap<String, PriceTable> = HashMap::new();
        overrides.insert("opus".to_string(), rt(1.0, 2.0, 3.0, 4.0));
        overrides.insert(
            "claude-opus-4-7".to_string(),
            rt(11.0, 22.0, 33.0, 44.0),
        );

        // Exact match wins.
        let r = lookup_rates("claude-opus-4-7", &overrides).unwrap();
        assert!((r.input - 11.0).abs() < f64::EPSILON);

        // Family fallback kicks in for an unknown opus variant.
        let r = lookup_rates("claude-opus-4-9-future", &overrides).unwrap();
        assert!((r.input - 1.0).abs() < f64::EPSILON);

        // No override for sonnet → hardcoded default.
        let r = lookup_rates("claude-sonnet-4-6", &overrides).unwrap();
        assert!((r.input - 3.0).abs() < f64::EPSILON);

        // Unknown family → None.
        assert!(lookup_rates("gpt-4", &overrides).is_none());
    }

    #[test]
    fn cost_combines_all_token_classes() {
        let rates = rt(10.0, 20.0, 30.0, 40.0);
        let row = UsageRow {
            input: 1_000_000,
            output: 500_000,
            cache_write: 250_000,
            cache_read: 2_000_000,
        };
        // 1*10 + 0.5*20 + 0.25*30 + 2*40 = 10 + 10 + 7.5 + 80 = 107.5
        assert!((cost(&rates, &row) - 107.5).abs() < 1e-9);
    }

    #[test]
    fn format_tokens_thresholds() {
        assert_eq!(format_tokens(42), "42");
        assert_eq!(format_tokens(2_500), "2K"); // {:.0} rounds to-nearest
        assert_eq!(format_tokens(1_400_000), "1.4M");
    }

    #[test]
    fn bucket_labels() {
        let ts = "2026-05-02T17:12:23.142Z";
        assert_eq!(bucket_label(ts, BucketMode::Daily).unwrap(), "2026-05-02");
        assert_eq!(bucket_label(ts, BucketMode::Monthly).unwrap(), "2026-05");
        // 2026-05-02 is a Saturday, which falls in ISO week 18 of 2026.
        assert_eq!(bucket_label(ts, BucketMode::Weekly).unwrap(), "2026-W18");
        // Malformed timestamps return None instead of panicking.
        assert!(bucket_label("not a date", BucketMode::Daily).is_none());
    }

    #[test]
    fn parse_jsonl_dedups_and_sums() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("session.jsonl");
        let line = |id: &str, req: &str, model: &str, input: u64, output: u64| {
            format!(
                r#"{{"requestId":"{req}","timestamp":"2026-05-02T17:12:23.142Z","message":{{"id":"{id}","model":"{model}","usage":{{"input_tokens":{input},"output_tokens":{output},"cache_creation_input_tokens":0,"cache_read_input_tokens":0}}}}}}"#,
            )
        };
        let content = format!(
            "{}\n{}\n{}\n{}\n",
            line("msg_a", "req_1", "claude-opus-4-7", 100, 50),
            // Duplicate of the previous line — should be ignored.
            line("msg_a", "req_1", "claude-opus-4-7", 100, 50),
            line("msg_b", "req_2", "claude-sonnet-4-6", 200, 80),
            // No usage block → user/tool message; should be skipped silently.
            r#"{"message":{"id":"msg_c","role":"user"}}"#,
        );
        std::fs::write(&path, content).unwrap();

        let mut seen = HashSet::new();
        let mut by_model = BTreeMap::new();
        parse_jsonl(&path, &mut seen, &mut by_model, None).unwrap();

        let opus = by_model.get("claude-opus-4-7").unwrap();
        assert_eq!(opus.input, 100);
        assert_eq!(opus.output, 50);

        let sonnet = by_model.get("claude-sonnet-4-6").unwrap();
        assert_eq!(sonnet.input, 200);
        assert_eq!(sonnet.output, 80);

        assert_eq!(by_model.len(), 2);
    }
}
