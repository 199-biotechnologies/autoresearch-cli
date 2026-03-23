use crate::errors::CliError;
use crate::git;
use crate::output::format::OutputFormat;

pub fn run(json: bool) -> Result<(), CliError> {
    let format = OutputFormat::detect(json);

    let initialized = std::path::Path::new("autoresearch.toml").exists();
    let is_git = git::is_git_repo();
    let loop_running = git::is_loop_running();
    let loop_state = git::loop_state();

    let branch = if initialized {
        load_branch()?.unwrap_or_else(|| "autoresearch".to_string())
    } else {
        "autoresearch".to_string()
    };

    let branch_exists = is_git && git::experiment_branch_exists(&branch);
    let current_branch = if is_git {
        git::current_branch().ok()
    } else {
        None
    };

    let experiment_count = if branch_exists {
        git::parse_experiments(&branch, 10000)
            .map(|e| e.len())
            .unwrap_or(0)
    } else {
        0
    };

    let best_metric = if branch_exists {
        git::parse_experiments(&branch, 10000)
            .ok()
            .and_then(|exps| {
                exps.iter()
                    .filter(|e| {
                        e.metric.is_some() && e.status == git::ExperimentStatus::Kept
                            || e.status == git::ExperimentStatus::Baseline
                    })
                    .filter_map(|e| e.metric)
                    .reduce(f64::min) // assumes lower is better by default
            })
    } else {
        None
    };

    match format {
        OutputFormat::Json => {
            let out = serde_json::json!({
                "status": "success",
                "data": {
                    "initialized": initialized,
                    "git_repo": is_git,
                    "branch": branch,
                    "branch_exists": branch_exists,
                    "current_branch": current_branch,
                    "experiment_count": experiment_count,
                    "best_metric": best_metric,
                    "loop_running": loop_running,
                    "loop_state": loop_state,
                }
            });
            println!("{}", serde_json::to_string_pretty(&out).unwrap());
        }
        OutputFormat::Table => {
            if !initialized {
                println!("Not initialized. Run `autoresearch init` to set up.");
                return Ok(());
            }

            println!("Autoresearch status:");
            println!("  Project:     initialized");
            println!(
                "  Branch:      {branch} {}",
                if branch_exists {
                    "(exists)"
                } else {
                    "(not created yet)"
                }
            );
            if let Some(cb) = &current_branch {
                println!("  Current:     {cb}");
            }
            println!("  Experiments: {experiment_count}");
            if let Some(bm) = best_metric {
                println!("  Best metric: {:.6}", bm);
            }
            println!(
                "  Loop:        {}",
                if loop_running {
                    "RUNNING"
                } else {
                    "not running"
                }
            );

            if let Some(state) = &loop_state {
                if let Some(iteration) = state.get("iteration").and_then(|i| i.as_u64()) {
                    println!("  Iteration:   #{iteration}");
                }
                if let Some(started) = state.get("started_at").and_then(|s| s.as_str()) {
                    println!("  Started:     {started}");
                }
            }
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
