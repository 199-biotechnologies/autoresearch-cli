use crate::errors::CliError;
use crate::git;
use crate::output::format::OutputFormat;
use std::process::Command;

pub fn run(names: &[String], json: bool) -> Result<(), CliError> {
    let format = OutputFormat::detect(json);

    if !git::is_git_repo() {
        return Err(CliError::NotGitRepo);
    }

    if names.is_empty() {
        return Err(CliError::Config(
            "Provide at least one fork name. Example: autoresearch fork 'try-transformers' 'try-convolutions'".into(),
        ));
    }

    let config = load_config()?;
    let base_branch = config
        .get("branch")
        .and_then(|v| v.as_str())
        .unwrap_or("autoresearch");

    // Determine the base point: current experiment branch or main
    let base_ref = if git::experiment_branch_exists(base_branch) {
        base_branch.to_string()
    } else {
        // Fall back to current branch
        git::current_branch()?
    };

    let mut created = Vec::new();

    for name in names {
        let branch_name = format!("autoresearch-fork-{name}");

        if git::experiment_branch_exists(&branch_name) {
            if let OutputFormat::Table = format {
                eprintln!("  warning: branch '{branch_name}' already exists, skipping");
            }
            created.push(serde_json::json!({
                "name": name,
                "branch": branch_name,
                "status": "already_exists",
            }));
            continue;
        }

        // Create branch from base
        let output = Command::new("git")
            .args(["branch", &branch_name, &base_ref])
            .output()
            .map_err(|e| CliError::Git(e.to_string()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(CliError::Git(format!(
                "Failed to create branch '{branch_name}': {stderr}"
            )));
        }

        // Copy experiments.jsonl to the fork if it exists
        // (each fork starts with the same history)
        created.push(serde_json::json!({
            "name": name,
            "branch": branch_name,
            "base": base_ref,
            "status": "created",
        }));
    }

    match format {
        OutputFormat::Json => {
            let out = serde_json::json!({
                "status": "success",
                "data": {
                    "base": base_ref,
                    "forks": created,
                },
                "suggestion": format!(
                    "Start agents on each fork: git checkout autoresearch/<name> then run /autoresearch"
                ),
            });
            println!("{}", serde_json::to_string_pretty(&out).unwrap());
        }
        OutputFormat::Table => {
            println!("Forked from '{base_ref}':\n");
            for fork in &created {
                let status = fork["status"].as_str().unwrap_or("");
                let branch = fork["branch"].as_str().unwrap_or("");
                let icon = if status == "created" { "+" } else { "~" };
                println!("  [{icon}] {branch} ({status})");
            }
            println!();
            println!("Start an agent on each fork:");
            for name in names {
                println!("  git checkout autoresearch-fork-{name} && /autoresearch");
            }
            println!();
            println!("Compare results later:");
            println!("  autoresearch status  (shows all fork branches)");
        }
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
