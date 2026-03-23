use thiserror::Error;

#[derive(Error, Debug)]
pub enum CliError {
    #[error("Not an autoresearch project (no autoresearch.toml found). Run `autoresearch init` first.")]
    NotInitialized,

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
            Self::NotInitialized | Self::Config(_) => 2,
            Self::NotGitRepo => 2,
            Self::NoExperiments(_) | Self::RunNotFound(_) => 1,
            Self::Git(_) => 1,
            Self::Io(_) => 1,
            Self::AlreadyInstalled(_) => 0,
            Self::ParseError(_) => 1,
        }
    }

    pub fn error_code(&self) -> &'static str {
        match self {
            Self::NotInitialized => "not_initialized",
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
}
