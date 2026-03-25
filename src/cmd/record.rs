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

    // Generate contextual hints based on experiment state
    let hints = generate_hints(&content, normalized_status, run_number, metric);

    match format {
        OutputFormat::Json => {
            let mut out = serde_json::json!({
                "status": "success",
                "data": record,
            });
            if !hints.is_empty() {
                out["hints"] = serde_json::json!(hints);
            }
            println!("{}", serde_json::to_string_pretty(&out).unwrap());
        }
        OutputFormat::Table => {
            println!(
                "Recorded experiment #{run_number}: metric={metric:.6} status={normalized_status} — {summary}"
            );
            for hint in &hints {
                eprintln!("hint: {hint}");
            }
        }
    }

    Ok(())
}

/// Generate contextual hints based on experiment history
fn generate_hints(jsonl_content: &str, status: &str, run_number: usize, current_metric: f64) -> Vec<String> {
    let mut hints = Vec::new();

    // Count recent consecutive discards
    let consecutive_discards = jsonl_content
        .lines()
        .rev()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| {
            serde_json::from_str::<serde_json::Value>(l)
                .ok()
                .and_then(|v| v.get("status").and_then(|s| s.as_str()).map(|s| s.to_string()))
        })
        .take_while(|s| s == "discarded")
        .count();

    let total_streak = if status == "discarded" {
        consecutive_discards + 1
    } else {
        0
    };

    // Stuck detection
    if total_streak >= 7 {
        hints.push(
            "STUCK: 7+ consecutive discards. You are in a deep local minimum. Try a fundamentally different approach: change strategy entirely, try removing code instead of adding, or run `autoresearch review` for cross-model analysis.".into()
        );
    } else if total_streak >= 5 {
        hints.push(
            "WARNING: 5+ consecutive discards. Consider: (1) run `autoresearch review` for fresh perspective, (2) try the OPPOSITE of recent attempts, (3) use `autoresearch fork` to explore multiple directions.".into()
        );
    } else if total_streak >= 3 {
        hints.push(
            "3 consecutive discards. Re-read `program.md` for ideas you haven't tried. Consider a different category of change (e.g., if tuning hyperparameters, try architecture instead).".into()
        );
    }

    // Baseline coaching
    if run_number == 0 && status == "baseline" {
        hints.push(
            "Baseline recorded. Strategy: start with hyperparameter tuning (learning rate, batch size, weight decay) — lowest risk, highest signal. Save architecture changes for later.".into()
        );
    }

    // Kept experiment: check for implausible improvement (Goodhart/reward hacking)
    if status == "kept" && run_number > 0 {
        let kept_metrics: Vec<f64> = jsonl_content
            .lines()
            .filter(|l| !l.trim().is_empty())
            .filter_map(|l| serde_json::from_str::<serde_json::Value>(l).ok())
            .filter(|v| {
                let s = v.get("status").and_then(|s| s.as_str()).unwrap_or("");
                s == "kept" || s == "baseline"
            })
            .filter_map(|v| v.get("metric").and_then(|m| m.as_f64()))
            .collect();

        if kept_metrics.len() >= 3 {
            let baseline = kept_metrics.first().copied().unwrap_or(1.0);
            let prev_best = kept_metrics.last().copied().unwrap_or(baseline);

            // Total cumulative improvement so far
            let cumulative = (baseline - prev_best).abs();
            // This experiment's claimed improvement
            let this_step = (prev_best - current_metric).abs();

            // If one step claims more than 10x the total cumulative improvement, flag it
            if cumulative > f64::EPSILON && this_step > cumulative * 10.0 {
                hints.push(format!(
                    "SUSPICIOUS: This single experiment claims {:.1}x more improvement than ALL previous experiments combined ({:.6} vs cumulative {:.6}). This may indicate reward hacking (e.g., caching I/O, memorizing the test set, gaming the eval). Verify the improvement is genuine before continuing. See: Goodhart's law of autoresearch.",
                    this_step / cumulative, this_step, cumulative
                ));
            }

            // Also flag if the metric improved by more than 100x in absolute terms
            if prev_best.abs() > f64::EPSILON {
                let ratio = current_metric / prev_best;
                if ratio > 100.0 || ratio < 0.01 {
                    hints.push(format!(
                        "SUSPICIOUS: Metric changed by {:.0}x in one step (from {:.6} to {:.6}). Typical autoresearch improvements are incremental. Large jumps often indicate the agent is gaming the eval rather than making real improvements.",
                        if ratio > 1.0 { ratio } else { 1.0 / ratio },
                        prev_best,
                        current_metric
                    ));
                }
            }
        }

        if hints.iter().all(|h| !h.starts_with("SUSPICIOUS")) {
            hints.push("Good improvement. Keep exploring in this direction with small variations before switching to a different approach.".into());
        }
    }

    // Periodic review reminder
    if run_number > 0 && run_number % 20 == 0 {
        hints.push(format!(
            "{run_number} experiments completed. Consider running `autoresearch report` to review progress and `autoresearch review` for cross-model analysis."
        ));
    }

    hints
}

fn load_config() -> Result<toml::Table, CliError> {
    let path = std::path::Path::new("autoresearch.toml");
    if !path.exists() {
        return Err(CliError::Config("No autoresearch.toml".into()));
    }
    let content = std::fs::read_to_string(path)?;
    toml::from_str(&content).map_err(|e| CliError::Config(e.to_string()))
}
