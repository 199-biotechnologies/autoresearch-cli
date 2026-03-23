use crate::cli::ExportFormat;
use crate::errors::CliError;
use crate::git;
use std::io::Write;

pub fn run(format: &ExportFormat, output: Option<&str>, _json: bool) -> Result<(), CliError> {
    let branch = load_branch()?.unwrap_or_else(|| "autoresearch".to_string());

    if !git::experiment_branch_exists(&branch) {
        return Err(CliError::NoExperiments(branch));
    }

    let experiments = git::parse_experiments(&branch, 10000)?;

    if experiments.is_empty() {
        return Err(CliError::NoExperiments(branch));
    }

    let content = match format {
        ExportFormat::Csv => export_csv(&experiments)?,
        ExportFormat::Json => serde_json::to_string_pretty(&experiments)
            .map_err(|e| CliError::ParseError(e.to_string()))?,
        ExportFormat::Jsonl => experiments
            .iter()
            .map(|e| serde_json::to_string(e).unwrap())
            .collect::<Vec<_>>()
            .join("\n"),
    };

    match output {
        Some(path) => {
            let mut file = std::fs::File::create(path)?;
            file.write_all(content.as_bytes())?;
            eprintln!("Exported {} experiments to {path}", experiments.len());
        }
        None => {
            print!("{content}");
        }
    }

    Ok(())
}

fn export_csv(experiments: &[git::Experiment]) -> Result<String, CliError> {
    let mut wtr = csv::Writer::from_writer(Vec::new());
    wtr.write_record(["run", "hash", "timestamp", "metric", "status", "summary"])
        .map_err(|e| CliError::ParseError(e.to_string()))?;

    for exp in experiments {
        wtr.write_record(&[
            exp.run.to_string(),
            exp.short_hash.clone(),
            exp.timestamp.clone(),
            exp.metric
                .map(|m| format!("{:.6}", m))
                .unwrap_or_default(),
            exp.status.to_string(),
            exp.summary.clone(),
        ])
        .map_err(|e| CliError::ParseError(e.to_string()))?;
    }

    let data = wtr
        .into_inner()
        .map_err(|e| CliError::ParseError(e.to_string()))?;
    String::from_utf8(data).map_err(|e| CliError::ParseError(e.to_string()))
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
