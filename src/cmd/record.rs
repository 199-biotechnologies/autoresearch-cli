use crate::errors::CliError;
use crate::output::format::OutputFormat;
use std::fs::{self, OpenOptions};
use std::io::Write;

pub fn run(metric: f64, status: &str, summary: &str, json: bool) -> Result<(), CliError> {
    let format = OutputFormat::detect(json);

    // Validate metric — reject NaN and Infinity
    if metric.is_nan() || metric.is_infinite() {
        return Err(CliError::Config(format!(
            "Invalid metric value '{}'. Must be a finite number, not NaN or Infinity.",
            metric
        )));
    }

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

    // Validate kept status against metric direction (anti-reward-hacking)
    if normalized_status == "kept" {
        if let Ok(config) = load_config() {
            let direction = config
                .get("metric_direction")
                .and_then(|v| v.as_str())
                .unwrap_or("lower");
            let lower_is_better = direction != "higher";

            // Find the last kept/baseline metric
            let log_path_check = std::path::Path::new(".autoresearch/experiments.jsonl");
            if log_path_check.exists() {
                if let Ok(content) = fs::read_to_string(log_path_check) {
                    let prev_best = content
                        .lines()
                        .rev()
                        .filter(|l| !l.trim().is_empty())
                        .filter_map(|l| serde_json::from_str::<serde_json::Value>(l).ok())
                        .filter(|v| {
                            let s = v.get("status").and_then(|s| s.as_str()).unwrap_or("");
                            s == "kept" || s == "baseline"
                        })
                        .find_map(|v| v.get("metric").and_then(|m| m.as_f64()));

                    if let Some(prev) = prev_best {
                        let regressed = if lower_is_better {
                            metric > prev
                        } else {
                            metric < prev
                        };
                        if regressed {
                            eprintln!(
                                "warning: metric {:.6} is worse than previous best {:.6} ({direction} is better). Recording as 'kept' anyway — consider 'discarded'.",
                                metric, prev
                            );
                        }
                    }
                }
            }
        }
    }

    let log_path = ".autoresearch/experiments.jsonl";
    fs::create_dir_all(".autoresearch")?;

    // Open and lock file FIRST, then compute run number inside the lock
    // This prevents race conditions where two concurrent records get the same run number
    let mut file = OpenOptions::new()
        .create(true)
        .read(true)
        .append(true)
        .open(log_path)?;

    // Acquire exclusive lock before reading or writing
    use std::os::unix::io::AsRawFd;
    unsafe {
        libc::flock(file.as_raw_fd(), libc::LOCK_EX);
    }

    // Now read existing content under lock to determine run number
    let content = fs::read_to_string(log_path)?;
    let run_number = content
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| {
            serde_json::from_str::<serde_json::Value>(l)
                .ok()
                .and_then(|v| v.get("run").and_then(|r| r.as_u64()))
        })
        .max()
        .map(|m| m as usize + 1)
        .unwrap_or(0);

    // Get current git hash
    let hash = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();

    let short_hash: String = hash.chars().take(7).collect();

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

    // Write under the lock we already hold (file opened + locked above)
    writeln!(file, "{}", serde_json::to_string(&record).unwrap())?;
    // Lock is released when file is dropped

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

fn load_config() -> Result<toml::Table, CliError> {
    let path = std::path::Path::new("autoresearch.toml");
    if !path.exists() {
        return Err(CliError::Config("No autoresearch.toml".into()));
    }
    let content = std::fs::read_to_string(path)?;
    toml::from_str(&content).map_err(|e| CliError::Config(e.to_string()))
}
