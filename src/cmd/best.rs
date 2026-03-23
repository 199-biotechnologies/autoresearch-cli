use crate::errors::CliError;
use crate::git;
use crate::output::format::OutputFormat;

pub fn run(json: bool) -> Result<(), CliError> {
    let format = OutputFormat::detect(json);
    let config = load_branch()?;
    let branch = config.as_deref().unwrap_or("autoresearch");

    if !git::experiment_branch_exists(branch) {
        return Err(CliError::NoExperiments(branch.to_string()));
    }

    let experiments = git::parse_experiments(branch, 500)?;
    if experiments.is_empty() {
        return Err(CliError::NoExperiments(branch.to_string()));
    }

    // Determine direction from config
    let direction = load_direction()?;
    let lower_is_better = direction.as_deref() != Some("higher");

    // Find best experiment (with a metric, status=kept or baseline)
    let best = experiments
        .iter()
        .filter(|e| {
            e.metric.is_some()
                && (e.status == git::ExperimentStatus::Kept
                    || e.status == git::ExperimentStatus::Baseline)
        })
        .min_by(|a, b| {
            let ma = a.metric.unwrap();
            let mb = b.metric.unwrap();
            if lower_is_better {
                ma.partial_cmp(&mb).unwrap()
            } else {
                mb.partial_cmp(&ma).unwrap()
            }
        });

    let baseline = experiments
        .iter()
        .find(|e| e.status == git::ExperimentStatus::Baseline);

    match (best, &format) {
        (Some(best), OutputFormat::Json) => {
            let mut data = serde_json::json!({
                "status": "success",
                "best": best,
            });
            if let Some(bl) = baseline {
                data["baseline"] = serde_json::json!(bl);
                if let (Some(bm), Some(blm)) = (best.metric, bl.metric) {
                    let improvement = if lower_is_better {
                        ((blm - bm) / blm) * 100.0
                    } else {
                        ((bm - blm) / blm) * 100.0
                    };
                    data["improvement_pct"] = serde_json::json!(improvement);
                }
            }
            println!("{}", serde_json::to_string_pretty(&data).unwrap());
        }
        (Some(best), OutputFormat::Table) => {
            println!("Best experiment:");
            println!("  Run:    #{}", best.run);
            println!("  Hash:   {}", best.short_hash);
            println!(
                "  Metric: {}",
                best.metric
                    .map(|m| format!("{:.6}", m))
                    .unwrap_or("-".into())
            );
            println!("  Summary: {}", best.summary);

            if let Some(bl) = baseline {
                if let (Some(bm), Some(blm)) = (best.metric, bl.metric) {
                    let improvement = if lower_is_better {
                        ((blm - bm) / blm) * 100.0
                    } else {
                        ((bm - blm) / blm) * 100.0
                    };
                    println!();
                    println!(
                        "  Baseline: {:.6} -> Best: {:.6} ({:.2}% improvement)",
                        blm, bm, improvement
                    );
                }
            }

            // Show the diff
            if !best.hash.is_empty() {
                println!();
                println!("Diff from parent:");
                match git::show_commit_diff(&best.hash) {
                    Ok(diff) => {
                        // Truncate long diffs
                        let lines: Vec<&str> = diff.lines().collect();
                        if lines.len() > 60 {
                            for line in &lines[..60] {
                                println!("  {line}");
                            }
                            println!("  ... ({} more lines)", lines.len() - 60);
                        } else {
                            for line in &lines {
                                println!("  {line}");
                            }
                        }
                    }
                    Err(_) => println!("  (could not retrieve diff)"),
                }
            }
        }
        (None, _) => {
            return Err(CliError::NoExperiments(branch.to_string()));
        }
    }

    Ok(())
}

fn load_branch() -> Result<Option<String>, CliError> {
    let path = std::path::Path::new("autoresearch.toml");
    if !path.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(path)?;
    let table: toml::Table =
        toml::from_str(&content).map_err(|e| CliError::Config(e.to_string()))?;
    Ok(table
        .get("branch")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string()))
}

fn load_direction() -> Result<Option<String>, CliError> {
    let path = std::path::Path::new("autoresearch.toml");
    if !path.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(path)?;
    let table: toml::Table =
        toml::from_str(&content).map_err(|e| CliError::Config(e.to_string()))?;
    Ok(table
        .get("metric_direction")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string()))
}
