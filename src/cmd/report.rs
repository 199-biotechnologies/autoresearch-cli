use crate::errors::CliError;
use crate::git;
use crate::output::format::OutputFormat;
use std::io::Write;

pub fn run(output_path: Option<&str>, json: bool) -> Result<(), CliError> {
    let format = OutputFormat::detect(json);

    // Load config
    let config = load_config()?;
    let branch = config
        .get("branch")
        .and_then(|v| v.as_str())
        .unwrap_or("autoresearch");
    let target_file = config
        .get("target_file")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    let metric_name = config
        .get("metric_name")
        .and_then(|v| v.as_str())
        .unwrap_or("metric");
    let metric_direction = config
        .get("metric_direction")
        .and_then(|v| v.as_str())
        .unwrap_or("lower");
    let eval_command = config
        .get("eval_command")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    // Load experiments
    let experiments = git::parse_experiments(branch, 10000)?;
    if experiments.is_empty() {
        return Err(CliError::NoExperiments(branch.to_string()));
    }

    let total = experiments.len();
    let kept: Vec<_> = experiments
        .iter()
        .filter(|e| e.status == git::ExperimentStatus::Kept)
        .collect();
    let discarded: Vec<_> = experiments
        .iter()
        .filter(|e| e.status == git::ExperimentStatus::Discarded)
        .collect();
    let baseline = experiments
        .iter()
        .find(|e| e.status == git::ExperimentStatus::Baseline);

    let lower_is_better = metric_direction != "higher";

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
                crate::git::safe_cmp(ma, mb)
            } else {
                crate::git::safe_cmp(mb, ma)
            }
        });

    let improvement_pct = match (best, baseline) {
        (Some(b), Some(bl)) => match (b.metric, bl.metric) {
            (Some(bm), Some(blm)) if blm != 0.0 => {
                let pct = if lower_is_better {
                    ((blm - bm) / blm) * 100.0
                } else {
                    ((bm - blm) / blm) * 100.0
                };
                Some(pct)
            }
            _ => None,
        },
        _ => None,
    };

    // Time span
    let first_ts = experiments.last().and_then(|e| {
        if e.timestamp.is_empty() {
            None
        } else {
            Some(e.timestamp.as_str())
        }
    });
    let last_ts = experiments.first().and_then(|e| {
        if e.timestamp.is_empty() {
            None
        } else {
            Some(e.timestamp.as_str())
        }
    });

    // Generate markdown report
    let mut report = String::new();

    report.push_str("# Autoresearch Report\n\n");

    report.push_str("## Summary\n\n");
    report.push_str(&format!("| Metric | Value |\n|--------|-------|\n"));
    report.push_str(&format!("| Target file | `{target_file}` |\n"));
    report.push_str(&format!("| Eval command | `{eval_command}` |\n"));
    report.push_str(&format!(
        "| Metric | {metric_name} ({metric_direction} is better) |\n"
    ));
    report.push_str(&format!("| Total experiments | {total} |\n"));
    report.push_str(&format!("| Kept | {} |\n", kept.len()));
    report.push_str(&format!("| Discarded | {} |\n", discarded.len()));

    if let Some(bl) = baseline {
        report.push_str(&format!(
            "| Baseline | {} |\n",
            bl.metric
                .map(|m| format!("{:.6}", m))
                .unwrap_or("-".into())
        ));
    }
    if let Some(b) = best {
        report.push_str(&format!(
            "| Best | {} (run #{}) |\n",
            b.metric
                .map(|m| format!("{:.6}", m))
                .unwrap_or("-".into()),
            b.run
        ));
    }
    if let Some(pct) = improvement_pct {
        report.push_str(&format!("| Improvement | {:.2}% |\n", pct));
    }
    if let (Some(first), Some(last)) = (first_ts, last_ts) {
        report.push_str(&format!("| Time span | {first} to {last} |\n"));
    }

    // Winning changes
    report.push_str("\n## Winning Changes (Kept)\n\n");
    if kept.is_empty() {
        report.push_str("No improvements found yet.\n");
    } else {
        for exp in &kept {
            report.push_str(&format!(
                "- **Run #{}** ({}): {} — {}\n",
                exp.run,
                exp.short_hash,
                exp.metric
                    .map(|m| format!("{:.6}", m))
                    .unwrap_or("-".into()),
                exp.summary
            ));
        }
    }

    // Failed attempts
    report.push_str("\n## Failed Attempts (Discarded)\n\n");
    if discarded.is_empty() {
        report.push_str("No failed experiments.\n");
    } else {
        for exp in &discarded {
            report.push_str(&format!(
                "- **Run #{}**: {} — {}\n",
                exp.run,
                exp.metric
                    .map(|m| format!("{:.6}", m))
                    .unwrap_or("-".into()),
                exp.summary
            ));
        }
    }

    // Metric progression
    report.push_str("\n## Metric Progression\n\n");
    report.push_str("```\n");
    let kept_and_baseline: Vec<_> = experiments
        .iter()
        .filter(|e| {
            e.status == git::ExperimentStatus::Kept
                || e.status == git::ExperimentStatus::Baseline
        })
        .collect();
    for exp in kept_and_baseline.iter().rev() {
        if let Some(m) = exp.metric {
            let bar_len = ((m * 40.0).min(80.0)).max(1.0) as usize;
            let bar: String = "#".repeat(bar_len.min(60));
            report.push_str(&format!("#{:>3} {:.4} {bar}\n", exp.run, m));
        }
    }
    report.push_str("```\n");

    report.push_str(&format!(
        "\n---\n*Generated by autoresearch CLI v{}*\n",
        env!("CARGO_PKG_VERSION")
    ));

    match format {
        OutputFormat::Json => {
            let out = serde_json::json!({
                "status": "success",
                "data": {
                    "report_markdown": report,
                    "summary": {
                        "total_experiments": total,
                        "kept": kept.len(),
                        "discarded": discarded.len(),
                        "baseline_metric": baseline.and_then(|b| b.metric),
                        "best_metric": best.and_then(|b| b.metric),
                        "improvement_pct": improvement_pct,
                    }
                }
            });
            println!("{}", serde_json::to_string_pretty(&out).unwrap());
        }
        OutputFormat::Table => match output_path {
            Some(path) => {
                let mut file = std::fs::File::create(path)?;
                file.write_all(report.as_bytes())?;
                eprintln!("Report written to {path}");
            }
            None => {
                print!("{report}");
            }
        },
    }

    Ok(())
}

fn load_config() -> Result<toml::Table, CliError> {
    let path = std::path::Path::new("autoresearch.toml");
    if !path.exists() {
        return Err(CliError::Config(
            "No autoresearch.toml found. Run `autoresearch init` first.".into(),
        ));
    }
    let content = std::fs::read_to_string(path)?;
    toml::from_str(&content).map_err(|e| CliError::Config(e.to_string()))
}
