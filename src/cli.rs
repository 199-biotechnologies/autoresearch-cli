use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser)]
#[command(
    name = "autoresearch",
    version,
    about = "Universal autoresearch CLI — optimize any metric with autonomous experiments",
    long_about = "Autoresearch CLI brings Karpathy's autoresearch pattern to any project.\n\n\
        Workflow: doctor → init → baseline → loop (hypothesize → implement → commit → eval → record) → report\n\n\
        Quick start:\n  \
        1. autoresearch install claude-code  (or codex, opencode, cursor, windsurf, all)\n  \
        2. autoresearch init --target-file <F> --eval-command <C>\n  \
        3. autoresearch doctor  (validate before starting)\n  \
        4. Tell your agent: /autoresearch\n\n\
        Best practices: hyperparameters first, one change per experiment, fork when stuck.\n\
        Run `autoresearch agent-info --json` for full machine-readable capabilities + strategy guide."
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
        /// Metric value from this experiment (use = for negatives: --metric=-0.5)
        #[arg(long, allow_negative_numbers = true)]
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

    /// Pre-flight check before starting an experiment loop
    Doctor,

    /// Fork experiments into parallel branches for multi-direction exploration
    Fork {
        /// Names for each fork (creates autoresearch/<name> branches)
        #[arg(required = true)]
        names: Vec<String>,
    },

    /// Generate a cross-model review prompt from experiment history
    Review,

    /// Live terminal dashboard — watch experiments as they happen
    Watch {
        /// Refresh interval in seconds
        #[arg(short, long, default_value = "2")]
        interval: u64,
    },

    /// Compare fork branches and merge the best one back
    MergeBest,

    /// Generate a markdown report of the research session
    Report {
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
