use crate::errors::CliError;
use crate::output::format::OutputFormat;
use std::fs::{self, OpenOptions};
use std::io::Write;

pub fn run(metric: f64, status: &str, summary: &str, json: bool) -> Result<(), CliError> {
    let format = OutputFormat::detect(json);

    // Validate status
    let valid_statuses = ["baseline", "kept", "keep", "discarded", "discard"];
    if !valid_statuses.contains(&status) {
        return Err(CliError::Config(format!(
            "Invalid status '{status}'. Must be one of: baseline, kept, discarded"
        )));
    }

    // Normalize status
    let normalized_status = match status {
        "keep" => "kept",
        "discard" => "discarded",
        _ => status,
    };

    // Read existing experiments to determine run number
    let log_path = ".autoresearch/experiments.jsonl";
    fs::create_dir_all(".autoresearch")?;

    let run_number = if std::path::Path::new(log_path).exists() {
        let content = fs::read_to_string(log_path)?;
        content
            .lines()
            .filter(|l| !l.trim().is_empty())
            .filter_map(|l| {
                serde_json::from_str::<serde_json::Value>(l)
                    .ok()
                    .and_then(|v| v.get("run").and_then(|r| r.as_u64()))
            })
            .max()
            .map(|m| m as usize + 1)
            .unwrap_or(0)
    } else {
        0
    };

    // Get current git hash
    let hash = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();

    let short_hash = if hash.len() >= 7 {
        hash[..7].to_string()
    } else {
        hash.clone()
    };

    // Calculate delta from previous kept/baseline metric
    let delta = if std::path::Path::new(log_path).exists() {
        let content = fs::read_to_string(log_path)?;
        let prev_metric = content
            .lines()
            .rev()
            .filter(|l| !l.trim().is_empty())
            .filter_map(|l| serde_json::from_str::<serde_json::Value>(l).ok())
            .filter(|v| {
                let s = v
                    .get("status")
                    .and_then(|s| s.as_str())
                    .unwrap_or("");
                s == "kept" || s == "baseline"
            })
            .find_map(|v| v.get("metric").and_then(|m| m.as_f64()));
        prev_metric.map(|prev| metric - prev)
    } else {
        None
    };

    let timestamp = chrono::Utc::now().to_rfc3339();

    let record = serde_json::json!({
        "run": run_number,
        "hash": hash,
        "short_hash": short_hash,
        "metric": metric,
        "delta": delta,
        "status": normalized_status,
        "summary": summary,
        "timestamp": timestamp,
    });

    // Append to JSONL file
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)?;
    writeln!(file, "{}", serde_json::to_string(&record).unwrap())?;

    match format {
        OutputFormat::Json => {
            let out = serde_json::json!({
                "status": "success",
                "data": record,
            });
            println!("{}", serde_json::to_string_pretty(&out).unwrap());
        }
        OutputFormat::Table => {
            println!(
                "Recorded experiment #{run_number}: metric={metric:.6} status={normalized_status} — {summary}"
            );
        }
    }

    Ok(())
}
