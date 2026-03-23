use thiserror::Error;

#[derive(Error, Debug)]
pub enum CliError {
    #[error("Not a git repository. Autoresearch requires git for experiment tracking.")]
    NotGitRepo,

    #[error("No experiments found on branch '{0}'.")]
    NoExperiments(String),

    #[error("Experiment run #{0} not found.")]
    RunNotFound(usize),

    #[error("Config error: {0}")]
    Config(String),

    #[error("Git error: {0}")]
    Git(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Skill already installed at {0}")]
    AlreadyInstalled(String),

    #[error("Failed to parse experiment log: {0}")]
    ParseError(String),
}

impl CliError {
    pub fn exit_code(&self) -> i32 {
        match self {
            Self::Config(_) | Self::NotGitRepo => 2,
            Self::NoExperiments(_) | Self::RunNotFound(_) => 1,
            Self::Git(_) => 1,
            Self::Io(_) => 1,
            Self::AlreadyInstalled(_) => 0,
            Self::ParseError(_) => 1,
        }
    }

    pub fn error_code(&self) -> &'static str {
        match self {
            Self::NotGitRepo => "not_git_repo",
            Self::NoExperiments(_) => "no_experiments",
            Self::RunNotFound(_) => "run_not_found",
            Self::Config(_) => "config_error",
            Self::Git(_) => "git_error",
            Self::Io(_) => "io_error",
            Self::AlreadyInstalled(_) => "already_installed",
            Self::ParseError(_) => "parse_error",
        }
    }

    pub fn suggestion(&self) -> &'static str {
        match self {
            Self::NotGitRepo => "Run `git init` first, then `autoresearch init`.",
            Self::NoExperiments(_) => {
                "Run `autoresearch init` then start the autoresearch loop in your agent."
            }
            Self::RunNotFound(_) => "Use `autoresearch log` to see available run numbers.",
            Self::Config(_) => "Check autoresearch.toml syntax. Regenerate with `autoresearch init`.",
            Self::Git(_) => "Check git state: `git status`. Ensure the experiment branch exists.",
            Self::Io(_) => "Check file permissions and disk space.",
            Self::AlreadyInstalled(_) => {
                "Already up to date. Update CLI version then reinstall for latest skill."
            }
            Self::ParseError(_) => {
                "Check .autoresearch/experiments.jsonl for malformed lines."
            }
        }
    }
}
