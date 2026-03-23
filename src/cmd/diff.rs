use crate::errors::CliError;
use crate::git;
use crate::output::format::OutputFormat;

pub fn run(run_a: usize, run_b: usize, json: bool) -> Result<(), CliError> {
    let format = OutputFormat::detect(json);
    let branch = load_branch()?.unwrap_or_else(|| "autoresearch".to_string());

    if !git::experiment_branch_exists(&branch) {
        return Err(CliError::NoExperiments(branch));
    }

    let experiments = git::parse_experiments(&branch, 500)?;

    let exp_a = experiments
        .iter()
        .find(|e| e.run == run_a)
        .ok_or(CliError::RunNotFound(run_a))?;
    let exp_b = experiments
        .iter()
        .find(|e| e.run == run_b)
        .ok_or(CliError::RunNotFound(run_b))?;

    if exp_a.hash.is_empty() || exp_b.hash.is_empty() {
        return Err(CliError::Git(
            "Cannot diff: missing commit hashes".to_string(),
        ));
    }

    let diff_output = git::diff_commits(&exp_a.hash, &exp_b.hash)?;

    match format {
        OutputFormat::Json => {
            let out = serde_json::json!({
                "status": "success",
                "data": {
                    "run_a": exp_a,
                    "run_b": exp_b,
                    "diff": diff_output,
                }
            });
            println!("{}", serde_json::to_string_pretty(&out).unwrap());
        }
        OutputFormat::Table => {
            println!("Comparing experiment #{run_a} vs #{run_b}:");
            println!();
            println!(
                "  #{run_a} ({}) metric={} — {}",
                exp_a.short_hash,
                exp_a
                    .metric
                    .map(|m| format!("{:.4}", m))
                    .unwrap_or("-".into()),
                exp_a.summary
            );
            println!(
                "  #{run_b} ({}) metric={} — {}",
                exp_b.short_hash,
                exp_b
                    .metric
                    .map(|m| format!("{:.4}", m))
                    .unwrap_or("-".into()),
                exp_b.summary
            );
            println!();

            if diff_output.is_empty() {
                println!("  No differences found.");
            } else {
                let lines: Vec<&str> = diff_output.lines().collect();
                if lines.len() > 80 {
                    for line in &lines[..80] {
                        println!("{line}");
                    }
                    println!("... ({} more lines)", lines.len() - 80);
                } else {
                    print!("{diff_output}");
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
