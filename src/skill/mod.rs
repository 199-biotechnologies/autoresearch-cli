pub mod templates;

use crate::cli::InstallTarget;
use crate::errors::CliError;
use std::fs;
use std::path::PathBuf;

/// Install the autoresearch skill for a specific target platform
pub fn install(target: &InstallTarget) -> Result<Vec<String>, CliError> {
    match target {
        InstallTarget::All => {
            let mut installed = Vec::new();
            let targets = [
                InstallTarget::ClaudeCode,
                InstallTarget::Codex,
                InstallTarget::Opencode,
                InstallTarget::Cursor,
                InstallTarget::Windsurf,
            ];
            for t in &targets {
                match install_single(t) {
                    Ok(path) => installed.push(path),
                    Err(CliError::AlreadyInstalled(p)) => {
                        installed.push(format!("{p} (already installed)"));
                    }
                    Err(e) => {
                        eprintln!("  warning: failed to install for {t:?}: {e}");
                    }
                }
            }
            Ok(installed)
        }
        other => {
            let path = install_single(other)?;
            Ok(vec![path])
        }
    }
}

fn install_single(target: &InstallTarget) -> Result<String, CliError> {
    let (dir, filename, content) = match target {
        InstallTarget::ClaudeCode => {
            let dir = home_dir()?.join(".claude/skills/autoresearch");
            let content = templates::claude_code_skill();
            (dir, "SKILL.md", content)
        }
        InstallTarget::Codex => {
            let dir = home_dir()?.join(".codex/skills/autoresearch");
            let content = templates::codex_skill();
            (dir, "SKILL.md", content)
        }
        InstallTarget::Opencode => {
            let dir = home_dir()?.join(".config/opencode/skills/autoresearch");
            let content = templates::opencode_skill();
            (dir, "SKILL.md", content)
        }
        InstallTarget::Cursor => {
            let dir = PathBuf::from(".cursor/rules");
            let content = templates::cursor_rule();
            (dir, "autoresearch.mdc", content)
        }
        InstallTarget::Windsurf => {
            let dir = PathBuf::from(".windsurf/rules");
            let content = templates::windsurf_rule();
            (dir, "autoresearch.md", content)
        }
        InstallTarget::All => unreachable!(),
    };

    let file_path = dir.join(filename);

    if file_path.exists() {
        // Check if it's the same version
        if let Ok(existing) = fs::read_to_string(&file_path) {
            if existing.contains(&format!("version: {}", env!("CARGO_PKG_VERSION"))) {
                return Err(CliError::AlreadyInstalled(
                    file_path.to_string_lossy().to_string(),
                ));
            }
        }
    }

    fs::create_dir_all(&dir)?;
    fs::write(&file_path, content)?;

    Ok(file_path.to_string_lossy().to_string())
}

fn home_dir() -> Result<PathBuf, CliError> {
    directories::BaseDirs::new()
        .map(|d| d.home_dir().to_path_buf())
        .ok_or_else(|| CliError::Config("Could not determine home directory".to_string()))
}
