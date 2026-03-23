use crate::errors::CliError;
use crate::git;
use crate::output::format::OutputFormat;
use comfy_table::{presets::UTF8_FULL_CONDENSED, Cell, Color, Table};

pub fn run(limit: usize, json: bool) -> Result<(), CliError> {
    let format = OutputFormat::detect(json);
    let config = load_config()?;
    let branch = config.branch.as_deref().unwrap_or("autoresearch");

    // Try to parse experiments — parse_experiments handles branch-aware JSONL reading
    // Don't reject on missing branch alone; JSONL may exist in working tree
    let experiments = git::parse_experiments(branch, limit).unwrap_or_default();

    if experiments.is_empty() {
        return Err(CliError::NoExperiments(branch.to_string()));
    }

    match format {
        OutputFormat::Json => {
            let out = serde_json::json!({
                "status": "success",
                "data": {
                    "branch": branch,
                    "total": experiments.len(),
                    "experiments": experiments,
                }
            });
            println!("{}", serde_json::to_string_pretty(&out).unwrap());
        }
        OutputFormat::Table => {
            let mut table = Table::new();
            table.load_preset(UTF8_FULL_CONDENSED);
            table.set_header(vec!["Run", "Hash", "Metric", "Delta", "Status", "Summary"]);

            let mut prev_metric: Option<f64> = None;
            for exp in &experiments {
                let delta = match (exp.metric, prev_metric) {
                    (Some(current), Some(prev)) => {
                        let d = current - prev;
                        if d > 0.0 {
                            format!("+{:.4}", d)
                        } else {
                            format!("{:.4}", d)
                        }
                    }
                    _ => "-".to_string(),
                };

                let status_cell = match exp.status {
                    git::ExperimentStatus::Kept => Cell::new("kept").fg(Color::Green),
                    git::ExperimentStatus::Discarded => Cell::new("discard").fg(Color::Red),
                    git::ExperimentStatus::Baseline => Cell::new("baseline").fg(Color::Cyan),
                    git::ExperimentStatus::Unknown => Cell::new("?").fg(Color::DarkGrey),
                };

                let metric_str = exp
                    .metric
                    .map(|m| format!("{:.4}", m))
                    .unwrap_or_else(|| "-".to_string());

                let summary = crate::output::truncate(&exp.summary, 50);

                table.add_row(vec![
                    Cell::new(exp.run),
                    Cell::new(&exp.short_hash),
                    Cell::new(&metric_str),
                    Cell::new(&delta),
                    status_cell,
                    Cell::new(&summary),
                ]);

                if exp.metric.is_some() {
                    prev_metric = exp.metric;
                }
            }

            println!("{table}");
        }
    }

    Ok(())
}

struct Config {
    branch: Option<String>,
}

fn load_config() -> Result<Config, CliError> {
    let path = std::path::Path::new("autoresearch.toml");
    if !path.exists() {
        // Graceful fallback — still try to work with git log
        return Ok(Config { branch: None });
    }

    let content = std::fs::read_to_string(path)?;
    let table: toml::Table =
        toml::from_str(&content).map_err(|e| CliError::Config(e.to_string()))?;

    Ok(Config {
        branch: table
            .get("branch")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
    })
}
