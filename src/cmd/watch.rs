use crate::errors::CliError;
use crate::git;
use std::io::Write;
use std::thread;
use std::time::Duration;

pub fn run(interval: u64) -> Result<(), CliError> {
    let config = load_config()?;
    let branch = config
        .get("branch")
        .and_then(|v| v.as_str())
        .unwrap_or("autoresearch");
    let metric_name = config
        .get("metric_name")
        .and_then(|v| v.as_str())
        .unwrap_or("metric");
    let metric_direction = config
        .get("metric_direction")
        .and_then(|v| v.as_str())
        .unwrap_or("lower");
    let target_file = config
        .get("target_file")
        .and_then(|v| v.as_str())
        .unwrap_or("?");

    let lower_is_better = metric_direction != "higher";
    let mut last_count = 0usize;

    loop {
        // Clear screen
        print!("\x1b[2J\x1b[H");
        std::io::stdout().flush().ok();

        println!(
            "\x1b[1;36m  AUTORESEARCH WATCH\x1b[0m  \x1b[90m({}s refresh, Ctrl+C to stop)\x1b[0m",
            interval
        );
        println!(
            "\x1b[90m  target: {target_file}  metric: {metric_name} ({metric_direction} is better)\x1b[0m"
        );
        println!();

        let loop_running = git::is_loop_running();
        let loop_state = git::loop_state();

        if loop_running {
            let iteration = loop_state
                .as_ref()
                .and_then(|s| s.get("iteration").and_then(|i| i.as_u64()))
                .unwrap_or(0);
            let started = loop_state
                .as_ref()
                .and_then(|s| s.get("started_at").and_then(|t| t.as_str()))
                .unwrap_or("?");
            println!(
                "  \x1b[1;32m● LOOP RUNNING\x1b[0m  iteration #{iteration}  started: {started}"
            );
        } else {
            println!("  \x1b[90m○ Loop not running\x1b[0m");
        }
        println!();

        // Load experiments
        let experiments = if git::experiment_branch_exists(branch) {
            git::parse_experiments(branch, 10000).unwrap_or_default()
        } else {
            vec![]
        };

        let total = experiments.len();
        let kept_count = experiments
            .iter()
            .filter(|e| e.status == git::ExperimentStatus::Kept)
            .count();
        let discarded_count = experiments
            .iter()
            .filter(|e| e.status == git::ExperimentStatus::Discarded)
            .count();

        let best_metric = experiments
            .iter()
            .filter(|e| {
                e.metric.is_some()
                    && (e.status == git::ExperimentStatus::Kept
                        || e.status == git::ExperimentStatus::Baseline)
            })
            .filter_map(|e| e.metric)
            .reduce(if lower_is_better {
                f64::min
            } else {
                f64::max
            });

        let baseline_metric = experiments
            .iter()
            .find(|e| e.status == git::ExperimentStatus::Baseline)
            .and_then(|e| e.metric);

        // Summary line
        println!(
            "  Experiments: \x1b[1m{total}\x1b[0m  \
             \x1b[32mKept: {kept_count}\x1b[0m  \
             \x1b[31mDiscarded: {discarded_count}\x1b[0m"
        );

        if let (Some(best), Some(bl)) = (best_metric, baseline_metric) {
            let improvement = if lower_is_better {
                ((bl - best) / bl) * 100.0
            } else {
                ((best - bl) / bl) * 100.0
            };
            println!(
                "  Baseline: {:.6}  Best: \x1b[1;32m{:.6}\x1b[0m  \
                 Improvement: \x1b[1;33m{:.2}%\x1b[0m",
                bl, best, improvement
            );
        } else if let Some(bl) = baseline_metric {
            println!("  Baseline: {:.6}  Best: -", bl);
        }

        // Spark line of kept metrics
        let kept_metrics: Vec<f64> = experiments
            .iter()
            .rev()
            .filter(|e| {
                e.metric.is_some()
                    && (e.status == git::ExperimentStatus::Kept
                        || e.status == git::ExperimentStatus::Baseline)
            })
            .filter_map(|e| e.metric)
            .collect();

        if kept_metrics.len() >= 2 {
            let min = kept_metrics
                .iter()
                .cloned()
                .reduce(f64::min)
                .unwrap_or(0.0);
            let max = kept_metrics
                .iter()
                .cloned()
                .reduce(f64::max)
                .unwrap_or(1.0);
            let range = max - min;
            let sparks = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

            let sparkline: String = kept_metrics
                .iter()
                .map(|v| {
                    if range == 0.0 {
                        sparks[4]
                    } else {
                        let normalized = ((v - min) / range * 7.0) as usize;
                        let idx = if lower_is_better {
                            7 - normalized.min(7)
                        } else {
                            normalized.min(7)
                        };
                        sparks[idx]
                    }
                })
                .collect();

            println!();
            println!("  Progress: \x1b[36m{sparkline}\x1b[0m");
        }

        // Recent experiments table
        println!();
        println!(
            "  \x1b[1m{:>4}  {:>10}  {:>8}  {:>10}  {}\x1b[0m",
            "Run", "Metric", "Delta", "Status", "Summary"
        );
        println!("  {}", "─".repeat(70));

        let recent: Vec<_> = experiments.iter().take(15).collect();
        // Display newest first
        for exp in &recent {
            let metric_str = exp
                .metric
                .map(|m| format!("{:.6}", m))
                .unwrap_or_else(|| "-".into());

            let delta_str = exp
                .metric
                .and_then(|m| {
                    baseline_metric.map(|bl| {
                        let d = m - bl;
                        if d > 0.0 {
                            format!("+{:.4}", d)
                        } else {
                            format!("{:.4}", d)
                        }
                    })
                })
                .unwrap_or_else(|| "-".into());

            let (status_str, color) = match exp.status {
                git::ExperimentStatus::Kept => ("kept", "\x1b[32m"),
                git::ExperimentStatus::Discarded => ("discard", "\x1b[31m"),
                git::ExperimentStatus::Baseline => ("baseline", "\x1b[36m"),
                git::ExperimentStatus::Unknown => ("?", "\x1b[90m"),
            };

            let summary = crate::output::truncate(&exp.summary, 35);

            println!(
                "  {color}{:>4}  {:>10}  {:>8}  {:>10}  {}\x1b[0m",
                exp.run, metric_str, delta_str, status_str, summary
            );
        }

        if total > 15 {
            println!("  \x1b[90m  ... {} more experiments\x1b[0m", total - 15);
        }

        // Check for new experiments
        if total > last_count && last_count > 0 {
            let new = total - last_count;
            println!();
            println!("  \x1b[1;33m  +{new} new experiment(s)\x1b[0m");
        }
        last_count = total;

        // Check fork branches
        let fork_branches = list_fork_branches();
        if !fork_branches.is_empty() {
            println!();
            println!("  \x1b[1mFork branches:\x1b[0m");
            for fb in &fork_branches {
                let count = git::parse_experiments(fb, 10000)
                    .map(|e| e.len())
                    .unwrap_or(0);
                println!("    {fb} ({count} experiments)");
            }
        }

        thread::sleep(Duration::from_secs(interval));
    }
}

fn list_fork_branches() -> Vec<String> {
    let output = std::process::Command::new("git")
        .args(["branch", "--list", "autoresearch-fork-*"])
        .output()
        .ok();

    match output {
        Some(o) if o.status.success() => String::from_utf8_lossy(&o.stdout)
            .lines()
            .map(|l| l.trim().trim_start_matches("* ").to_string())
            .filter(|l| !l.is_empty())
            .collect(),
        _ => vec![],
    }
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
