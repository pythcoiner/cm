//! `cm update-pricing` subcommand.
//!
//! Fetches the latest Claude/Anthropic model rates from LiteLLM and writes
//! them into the `[pricing]` table of `.cm/config.toml`. Other keys,
//! comments, and field ordering in the config file are preserved by using
//! `toml_edit` for surgical editing.

use std::collections::BTreeMap;
use std::path::Path;

use log::info;
use serde_json::Value;
use thiserror::Error;
use toml_edit::{DocumentMut, Item, Table, Value as TomlValue};

const LITELLM_URL: &str =
    "https://raw.githubusercontent.com/BerriAI/litellm/main/model_prices_and_context_window.json";

const FAMILIES: &[&str] = &["opus", "sonnet", "haiku"];

#[derive(Debug, Error)]
pub enum UpdatePricingError {
    #[error("io error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("could not resolve current working directory: {0}")]
    CwdNotFound(std::io::Error),

    #[error("`.cm/` directory not found in current project; run `cm init` first")]
    CmDirMissing,

    #[error("HTTP error fetching {url}: {source}")]
    Http {
        url: String,
        #[source]
        source: Box<ureq::Error>,
    },

    #[error("failed to parse upstream pricing JSON: {0}")]
    Json(serde_json::Error),

    #[error("failed to parse existing config.toml: {0}")]
    TomlParse(toml_edit::TomlError),
}

/// Per-model rates, ready to write into TOML ($/1M tokens).
#[derive(Debug, Clone, Copy)]
struct Rates {
    input: f64,
    output: f64,
    cache_write: f64,
    cache_read: f64,
}

#[derive(Debug, Default)]
pub struct UpdatePricingOpts {
    /// Print what would change without modifying the file.
    pub dry_run: bool,
}

pub fn execute_update_pricing(opts: UpdatePricingOpts) -> Result<(), UpdatePricingError> {
    let cwd = std::env::current_dir().map_err(UpdatePricingError::CwdNotFound)?;
    let cm_dir = cwd.join(".cm");
    if !cm_dir.is_dir() {
        return Err(UpdatePricingError::CmDirMissing);
    }
    let config_path = cm_dir.join("config.toml");

    println!("Fetching pricing data from LiteLLM …");
    let raw = fetch_litellm()?;
    let rates = extract_anthropic_rates(&raw);
    if rates.is_empty() {
        println!("No Anthropic pricing entries found in upstream JSON; nothing to update.");
        return Ok(());
    }
    let families = pick_family_representatives(&rates);

    println!(
        "Discovered {} model entries ({} family fallback{}).",
        rates.len(),
        families.len(),
        if families.len() == 1 { "" } else { "s" },
    );

    if opts.dry_run {
        println!("Dry run — would write the following pricing table:");
        print_summary(&rates, &families);
        return Ok(());
    }

    update_config_file(&config_path, &rates, &families)?;
    info!("wrote pricing table to {}", config_path.display());
    println!("Updated {}", config_path.display());
    print_summary(&rates, &families);

    Ok(())
}

fn fetch_litellm() -> Result<Value, UpdatePricingError> {
    let response = ureq::get(LITELLM_URL)
        .call()
        .map_err(|e| UpdatePricingError::Http {
            url: LITELLM_URL.to_string(),
            source: Box::new(e),
        })?;
    let body = response.into_string()?;
    serde_json::from_str(&body).map_err(UpdatePricingError::Json)
}

fn extract_anthropic_rates(root: &Value) -> BTreeMap<String, Rates> {
    let mut out = BTreeMap::new();
    let Some(map) = root.as_object() else { return out };

    for (key, val) in map {
        if key == "sample_spec" || !key.starts_with("claude-") {
            continue;
        }
        if val.get("litellm_provider").and_then(Value::as_str) != Some("anthropic") {
            continue;
        }
        let Some(rates) = read_rates(val) else { continue };
        out.insert(key.clone(), rates);
    }
    out
}

fn read_rates(val: &Value) -> Option<Rates> {
    let input = read_cost(val, "input_cost_per_token")?;
    let output = read_cost(val, "output_cost_per_token")?;
    // Cache fields are optional in LiteLLM data — fall back to 0 if absent.
    let cache_write = read_cost(val, "cache_creation_input_token_cost").unwrap_or(0.0);
    let cache_read = read_cost(val, "cache_read_input_token_cost").unwrap_or(0.0);

    if input <= 0.0 || output <= 0.0 {
        return None;
    }

    // LiteLLM stores per-token rates; convert to $/1M.
    Some(Rates {
        input: round4(input * 1_000_000.0),
        output: round4(output * 1_000_000.0),
        cache_write: round4(cache_write * 1_000_000.0),
        cache_read: round4(cache_read * 1_000_000.0),
    })
}

fn read_cost(val: &Value, key: &str) -> Option<f64> {
    val.get(key).and_then(Value::as_f64)
}

fn round4(x: f64) -> f64 {
    (x * 10_000.0).round() / 10_000.0
}

/// Select a representative model per family for fallback entries. The
/// lexicographically largest exact model name within each family wins —
/// for current Claude naming this picks the highest version (e.g.
/// `claude-opus-4-7` > `claude-opus-4-5`).
fn pick_family_representatives(rates: &BTreeMap<String, Rates>) -> BTreeMap<&'static str, Rates> {
    let mut out: BTreeMap<&'static str, Rates> = BTreeMap::new();
    for fam in FAMILIES {
        let mut latest: Option<(&str, Rates)> = None;
        for (name, r) in rates {
            if !name.contains(fam) {
                continue;
            }
            match &latest {
                None => latest = Some((name.as_str(), *r)),
                Some((cur, _)) if name.as_str() > *cur => latest = Some((name.as_str(), *r)),
                _ => {}
            }
        }
        if let Some((_, r)) = latest {
            out.insert(*fam, r);
        }
    }
    out
}

fn update_config_file(
    path: &Path,
    rates: &BTreeMap<String, Rates>,
    families: &BTreeMap<&'static str, Rates>,
) -> Result<(), UpdatePricingError> {
    let original = if path.exists() {
        std::fs::read_to_string(path)?
    } else {
        String::new()
    };

    let mut doc: DocumentMut = original
        .parse::<DocumentMut>()
        .map_err(UpdatePricingError::TomlParse)?;

    // Replace the `pricing` table wholesale; non-pricing keys are untouched.
    let mut pricing = Table::new();
    pricing.set_implicit(true);

    for (name, r) in families {
        pricing.insert((*name).to_string().as_str(), Item::Table(rates_to_table(r)));
    }
    for (name, r) in rates {
        pricing.insert(name.as_str(), Item::Table(rates_to_table(r)));
    }

    doc["pricing"] = Item::Table(pricing);

    // Ensure parent directory exists (true by construction since we checked
    // `.cm/`, but be defensive).
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, doc.to_string())?;
    Ok(())
}

fn rates_to_table(r: &Rates) -> Table {
    let mut t = Table::new();
    t.insert("input", Item::Value(TomlValue::from(r.input)));
    t.insert("output", Item::Value(TomlValue::from(r.output)));
    t.insert("cache_write", Item::Value(TomlValue::from(r.cache_write)));
    t.insert("cache_read", Item::Value(TomlValue::from(r.cache_read)));
    t
}

fn print_summary(
    rates: &BTreeMap<String, Rates>,
    families: &BTreeMap<&'static str, Rates>,
) {
    println!();
    println!("Family fallbacks:");
    for (fam, r) in families {
        println!(
            "  {fam:<8}  in: ${:.2}/M  out: ${:.2}/M  cache_w: ${:.2}/M  cache_r: ${:.2}/M",
            r.input, r.output, r.cache_write, r.cache_read,
        );
    }
    println!();
    println!("Models ({}):", rates.len());
    for (name, r) in rates {
        println!(
            "  {name:<40}  in: ${:.2}/M  out: ${:.2}/M",
            r.input, r.output,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn extract_filters_to_anthropic_claude_models() {
        let root = json!({
            "sample_spec": {"input_cost_per_token": 1.0},
            "claude-opus-4-7": {
                "litellm_provider": "anthropic",
                "input_cost_per_token": 0.000015,
                "output_cost_per_token": 0.000075,
                "cache_creation_input_token_cost": 0.00001875,
                "cache_read_input_token_cost": 0.0000015,
            },
            "claude-opus-on-bedrock": {
                "litellm_provider": "bedrock",
                "input_cost_per_token": 0.000015,
                "output_cost_per_token": 0.000075,
            },
            "gpt-4": {
                "litellm_provider": "openai",
                "input_cost_per_token": 0.00003,
                "output_cost_per_token": 0.00006,
            },
        });
        let rates = extract_anthropic_rates(&root);
        assert_eq!(rates.len(), 1);
        let r = rates.get("claude-opus-4-7").unwrap();
        assert!((r.input - 15.0).abs() < 1e-6);
        assert!((r.output - 75.0).abs() < 1e-6);
        assert!((r.cache_write - 18.75).abs() < 1e-6);
        assert!((r.cache_read - 1.5).abs() < 1e-6);
    }

    #[test]
    fn family_fallback_picks_highest_named() {
        let mut rates = BTreeMap::new();
        rates.insert(
            "claude-opus-4-5".to_string(),
            Rates {
                input: 1.0,
                output: 2.0,
                cache_write: 3.0,
                cache_read: 4.0,
            },
        );
        rates.insert(
            "claude-opus-4-7".to_string(),
            Rates {
                input: 10.0,
                output: 20.0,
                cache_write: 30.0,
                cache_read: 40.0,
            },
        );
        let families = pick_family_representatives(&rates);
        let opus = families.get("opus").unwrap();
        assert!((opus.input - 10.0).abs() < f64::EPSILON);
    }

    #[test]
    fn update_preserves_unrelated_keys_and_replaces_pricing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(
            &path,
            r#"
# user comment
model = "claude-sonnet-4-6"
max_cycles = 7

[pricing.opus]
input = 999.0
output = 999.0
cache_write = 999.0
cache_read = 999.0
"#,
        )
        .unwrap();

        let mut rates = BTreeMap::new();
        rates.insert(
            "claude-opus-4-7".to_string(),
            Rates {
                input: 15.0,
                output: 75.0,
                cache_write: 18.75,
                cache_read: 1.50,
            },
        );
        let families = pick_family_representatives(&rates);
        update_config_file(&path, &rates, &families).unwrap();

        let new = std::fs::read_to_string(&path).unwrap();
        // Non-pricing keys are preserved.
        assert!(new.contains("# user comment"));
        assert!(new.contains("model = \"claude-sonnet-4-6\""));
        assert!(new.contains("max_cycles = 7"));
        // Old stale rate is gone, new rate present.
        assert!(!new.contains("999.0"));
        assert!(new.contains("input = 15.0"));
        // toml_edit emits bare keys when permitted (dashes are legal); both
        // tables must appear and the file must round-trip via the cm config
        // loader.
        assert!(new.contains("[pricing.claude-opus-4-7]"));
        assert!(new.contains("[pricing.opus]"));

        let parsed: crate::config::ConfigFile = toml::from_str(&new).unwrap();
        let pricing = parsed.pricing.expect("pricing round-trips");
        assert!(pricing.contains_key("claude-opus-4-7"));
        assert!(pricing.contains_key("opus"));
    }
}
