use crate::errors::CliError;
use crate::git;
use crate::output::format::OutputFormat;
use std::process::Command;

pub fn run(json: bool) -> Result<(), CliError> {
    let format = OutputFormat::detect(json);

    if !git::is_git_repo() {
        return Err(CliError::NotGitRepo);
    }

    let config = load_config()?;
    let base_branch = config
        .get("branch")
        .and_then(|v| v.as_str())
        .unwrap_or("autoresearch");
    let metric_direction = config
        .get("metric_direction")
        .and_then(|v| v.as_str())
        .unwrap_or("lower");
    let lower_is_better = metric_direction != "higher";

    // Find all fork branches
    let fork_branches = list_fork_branches();

    // Also include the main experiment branch
    let mut all_branches: Vec<String> = vec![];
    if git::experiment_branch_exists(base_branch) {
        all_branches.push(base_branch.to_string());
    }
    all_branches.extend(fork_branches);

    if all_branches.len() < 2 {
        return Err(CliError::Config(
            "Need at least 2 branches to compare. Use `autoresearch fork` to create branches first."
                .into(),
        ));
    }

    // For each branch, find the best metric
    let mut branch_results: Vec<BranchResult> = Vec::new();

    for branch in &all_branches {
        let experiments = git::parse_experiments(branch, 10000).unwrap_or_default();
        let total = experiments.len();
        let kept = experiments
            .iter()
            .filter(|e| e.status == git::ExperimentStatus::Kept)
            .count();

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

        branch_results.push(BranchResult {
            branch: branch.clone(),
            total_experiments: total,
            kept_experiments: kept,
            best_metric: best.and_then(|e| e.metric),
            best_run: best.map(|e| e.run).unwrap_or(0),
            _best_hash: best.map(|e| e.hash.clone()).unwrap_or_default(),
            best_summary: best.map(|e| e.summary.clone()).unwrap_or_default(),
        });
    }

    // Sort by best metric
    branch_results.sort_by(|a, b| {
        match (a.best_metric, b.best_metric) {
            (Some(ma), Some(mb)) => {
                if lower_is_better {
                    crate::git::safe_cmp(ma, mb)
                } else {
                    crate::git::safe_cmp(mb, ma)
                }
            }
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => std::cmp::Ordering::Equal,
        }
    });

    let winner = &branch_results[0];

    match format {
        OutputFormat::Json => {
            let out = serde_json::json!({
                "status": "success",
                "data": {
                    "winner": {
                        "branch": winner.branch,
                        "best_metric": winner.best_metric,
                        "best_run": winner.best_run,
                        "total_experiments": winner.total_experiments,
                        "kept_experiments": winner.kept_experiments,
                        "best_summary": winner.best_summary,
                    },
                    "all_branches": branch_results.iter().map(|br| {
                        serde_json::json!({
                            "branch": br.branch,
                            "best_metric": br.best_metric,
                            "total_experiments": br.total_experiments,
                            "kept_experiments": br.kept_experiments,
                        })
                    }).collect::<Vec<_>>(),
                },
                "suggestion": format!(
                    "To merge the winner: git checkout {base_branch} && git merge {}",
                    winner.branch
                ),
            });
            println!("{}", serde_json::to_string_pretty(&out).unwrap());
        }
        OutputFormat::Table => {
            println!("Branch Comparison ({metric_direction} is better):\n");
            println!(
                "  {:>4}  {:<30}  {:>10}  {:>6}  {:>6}",
                "Rank", "Branch", "Best", "Kept", "Total"
            );
            println!("  {}", "─".repeat(65));

            for (i, br) in branch_results.iter().enumerate() {
                let metric_str = br
                    .best_metric
                    .map(|m| format!("{:.6}", m))
                    .unwrap_or_else(|| "-".into());
                let marker = if i == 0 { " *" } else { "" };
                let branch_display = crate::output::truncate(&br.branch, 28);

                println!(
                    "  {:>4}  {:<30}  {:>10}  {:>6}  {:>6}{}",
                    i + 1,
                    branch_display,
                    metric_str,
                    br.kept_experiments,
                    br.total_experiments,
                    marker,
                );
            }

            println!();
            println!(
                "Winner: \x1b[1;32m{}\x1b[0m (best: {})",
                winner.branch,
                winner
                    .best_metric
                    .map(|m| format!("{:.6}", m))
                    .unwrap_or("-".into())
            );
            println!();
            println!("To merge the winner into {base_branch}:");
            println!(
                "  git checkout {base_branch} && git merge {}",
                winner.branch
            );
            println!();
            println!("To clean up losing branches:");
            for br in branch_results.iter().skip(1) {
                if br.branch != base_branch {
                    println!("  git branch -D {}", br.branch);
                }
            }
        }
    }

    Ok(())
}

struct BranchResult {
    branch: String,
    total_experiments: usize,
    kept_experiments: usize,
    best_metric: Option<f64>,
    best_run: usize,
    _best_hash: String,
    best_summary: String,
}

fn list_fork_branches() -> Vec<String> {
    let output = Command::new("git")
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
