use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser)]
#[command(
    name = "autoresearch",
    version,
    about = "Universal autoresearch CLI — install skills, track experiments, view results",
    long_about = "Autoresearch CLI brings Karpathy's autoresearch pattern to any project.\n\n\
        Install the autoresearch loop skill into any AI coding agent (Claude Code, Codex, \
        OpenCode, Cursor, Windsurf), scaffold experiments, and track results from the terminal."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Output as JSON (auto-enabled when piped)
    #[arg(long, global = true)]
    pub json: bool,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Install the autoresearch skill into an AI coding agent
    Install {
        /// Target agent platform
        #[arg(value_enum)]
        target: InstallTarget,
    },

    /// Initialize autoresearch in the current project
    Init {
        /// Target file the agent may modify
        #[arg(long)]
        target_file: Option<String>,

        /// Eval command that produces the metric
        #[arg(long)]
        eval_command: Option<String>,

        /// Metric name (e.g., val_bpb, accuracy, p99_latency)
        #[arg(long, default_value = "metric")]
        metric_name: String,

        /// Metric direction: lower or higher
        #[arg(long, default_value = "lower")]
        metric_direction: String,

        /// Time budget per experiment (e.g., 5m, 30s)
        #[arg(long, default_value = "5m")]
        time_budget: String,

        /// Git branch for experiments
        #[arg(long, default_value = "autoresearch")]
        branch: String,
    },

    /// Record an experiment result (for agent use)
    Record {
        /// Metric value from this experiment
        #[arg(long)]
        metric: f64,

        /// Status: kept, discarded, or baseline
        #[arg(long)]
        status: String,

        /// Summary of what was tried
        #[arg(long)]
        summary: String,
    },

    /// Show experiment history from git log
    Log {
        /// Maximum number of entries to show
        #[arg(short = 'n', long, default_value = "20")]
        limit: usize,
    },

    /// Show the best experiment and its diff from baseline
    Best,

    /// Compare two experiments by run number
    Diff {
        /// First run number
        run_a: usize,
        /// Second run number
        run_b: usize,
    },

    /// Check if an autoresearch loop is currently running
    Status,

    /// Export experiment history
    Export {
        /// Export format
        #[arg(long, value_enum, default_value = "csv")]
        format: ExportFormat,

        /// Output file path (stdout if not specified)
        #[arg(short, long)]
        output: Option<String>,
    },

    /// Show CLI capabilities for agent discovery
    AgentInfo,
}

#[derive(ValueEnum, Clone, Debug)]
pub enum InstallTarget {
    /// Claude Code (~/.claude/skills/)
    ClaudeCode,
    /// Codex CLI (~/.codex/skills/)
    Codex,
    /// OpenCode (~/.config/opencode/skills/)
    Opencode,
    /// Cursor (.cursor/rules/)
    Cursor,
    /// Windsurf (.windsurf/rules/)
    Windsurf,
    /// Install into all supported agents
    All,
}

#[derive(ValueEnum, Clone, Debug)]
pub enum ExportFormat {
    Csv,
    Json,
    Jsonl,
}

pub fn parse() -> Cli {
    Cli::parse()
}
