use crate::errors::CliError;
use crate::output::format::OutputFormat;

pub fn run(json: bool) -> Result<(), CliError> {
    let format = OutputFormat::detect(json);

    let info = serde_json::json!({
        "name": "autoresearch",
        "version": env!("CARGO_PKG_VERSION"),
        "description": "Universal autoresearch CLI — install skills, track experiments, view results",
        "commands": {
            "install": "Install the autoresearch skill into an AI coding agent (claude-code, codex, opencode, cursor, windsurf, all)",
            "init": "Initialize autoresearch in the current project (--target-file, --eval-command, --metric-name, --metric-direction, --time-budget, --branch)",
            "record": "Record an experiment result (--metric <V> --status <S> --summary <M>). Handles JSONL, run numbering, deltas.",
            "doctor": "Pre-flight check: validates git, config, eval command, target file, branch state before starting a loop",
            "log": "Show experiment history from git log (supports -n limit)",
            "best": "Show the best experiment and its diff from baseline",
            "diff": "Compare two experiments by run number (diff <run_a> <run_b>)",
            "status": "Check project state and whether a loop is running",
            "export": "Export experiment history (--format csv|json|jsonl, -o file)",
            "report": "Generate markdown report of the research session (-o file)",
            "agent-info": "This command — machine-readable capability metadata",
        },
        "supported_targets": ["claude-code", "codex", "opencode", "cursor", "windsurf"],
        "config_file": "autoresearch.toml",
        "experiment_log": ".autoresearch/experiments.jsonl",
        "global_flags": ["--json", "--help", "--version"],
    });

    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&info).unwrap());
        }
        OutputFormat::Table => {
            println!("autoresearch v{}", env!("CARGO_PKG_VERSION"));
            println!();
            println!("Commands:");
            if let Some(cmds) = info.get("commands").and_then(|c| c.as_object()) {
                for (name, desc) in cmds {
                    println!("  {name:12} — {}", desc.as_str().unwrap_or(""));
                }
            }
            println!();
            println!("Targets: claude-code, codex, opencode, cursor, windsurf, all");
            println!();
            println!("Use --json on any command for machine-readable output.");
        }
    }

    Ok(())
}
