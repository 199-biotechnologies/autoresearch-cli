use crate::errors::CliError;
use crate::output::format::OutputFormat;

pub fn run(json: bool) -> Result<(), CliError> {
    let format = OutputFormat::detect(json);

    let info = serde_json::json!({
        "name": "autoresearch",
        "version": env!("CARGO_PKG_VERSION"),
        "description": "Universal autoresearch CLI — install skills, track experiments, optimize any measurable metric",
        "commands": {
            "install <target>": "Install skill into an AI agent (claude-code, codex, opencode, cursor, windsurf, all)",
            "init": "Scaffold project (--target-file, --eval-command, --metric-name, --metric-direction, --time-budget, --branch)",
            "doctor": "Pre-flight check (14 checks). Run FIRST before any experiment loop.",
            "record": "Record experiment (--metric <V> --status kept|discarded|baseline --summary '<M>'). Handles JSONL, run numbering, deltas, reward-hacking detection.",
            "log [-n N]": "Show experiment history with metrics and status",
            "best": "Show best experiment + diff from baseline + improvement %",
            "diff <a> <b>": "Compare two experiments by run number",
            "status": "Project state, best metric, loop running status",
            "export": "Export history (--format csv|json|jsonl, -o file)",
            "fork <names...>": "Create parallel exploration branches from current best",
            "merge-best": "Compare all fork branches, rank by metric, identify winner",
            "review": "Generate cross-model review prompt with stuck detection and failure pattern analysis",
            "watch": "Live terminal dashboard (-i seconds for refresh interval)",
            "report [-o file]": "Generate markdown research report",
            "agent-info": "This command — capabilities + best practices for agents",
        },
        "workflow": {
            "step_1": "autoresearch doctor (validate environment)",
            "step_2": "Read autoresearch.toml + program.md",
            "step_3": "git checkout -b <branch>",
            "step_4": "Record baseline: autoresearch record --metric <V> --status baseline --summary 'Initial baseline'",
            "step_5": "Loop: hypothesize → implement → commit → eval → record (kept/discarded) → repeat",
            "step_6": "When done: autoresearch report",
        },
        "best_practices": {
            "experiment_order": [
                "1. Hyperparameters first (learning rate, batch size, weight decay) — lowest risk, highest signal",
                "2. Regularization second (dropout, decay schedules, gradient clipping)",
                "3. Architecture changes third (high variance — most fail, but winners are big)",
                "4. Exotic/novel ideas last (papers, unconventional techniques)",
            ],
            "when_stuck": [
                "After 5+ consecutive discards, you are in a local minimum.",
                "Run: autoresearch review (generates cross-model analysis prompt)",
                "Try the OPPOSITE of what you've been doing",
                "Remove something — the best optimization is often removal",
                "Fork with: autoresearch fork approach-a approach-b",
            ],
            "anti_patterns": [
                "DO NOT combine multiple changes in one experiment",
                "DO NOT skip the commit before eval",
                "DO NOT write to experiments.jsonl directly — use autoresearch record",
                "DO NOT repeat a failed approach without a new angle",
                "DO NOT ignore the reward-hacking warning from record",
            ],
            "reward_hacking": "The metric can lie. Watch for overfitting the eval, secondary costs (time/memory), and changes that game the metric without real improvement.",
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
                    println!("  {name:20} {}", desc.as_str().unwrap_or(""));
                }
            }
            println!();
            println!("Workflow: doctor → init → baseline → loop (hypothesize → implement → commit → eval → record) → report");
            println!();
            println!("Best practices:");
            println!("  1. Hyperparameters first, architecture last");
            println!("  2. One atomic change per experiment");
            println!("  3. After 5+ discards → run `autoresearch review` or fork");
            println!("  4. The metric can lie — watch for reward hacking");
            println!("  5. The best optimization is often removal");
            println!();
            println!("Use --json for full machine-readable output with all best practices.");
        }
    }

    Ok(())
}
