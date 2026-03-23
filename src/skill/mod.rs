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
                InstallTarget::Gemini,
                InstallTarget::Codex,
                InstallTarget::Opencode,
                InstallTarget::Copilot,
                InstallTarget::Cursor,
                InstallTarget::Windsurf,
                InstallTarget::Agents,
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
            let content = templates::skill_md("claude-code");
            (dir, "SKILL.md", content)
        }
        InstallTarget::Gemini => {
            let dir = home_dir()?.join(".gemini/skills/autoresearch");
            let content = templates::skill_md("gemini");
            (dir, "SKILL.md", content)
        }
        InstallTarget::Codex => {
            let dir = home_dir()?.join(".codex/skills/autoresearch");
            let content = templates::skill_md("codex");
            (dir, "SKILL.md", content)
        }
        InstallTarget::Opencode => {
            let dir = home_dir()?.join(".config/opencode/skills/autoresearch");
            let content = templates::skill_md("opencode");
            (dir, "SKILL.md", content)
        }
        InstallTarget::Copilot => {
            let dir = PathBuf::from(".github/skills/autoresearch");
            let content = templates::skill_md("copilot");
            (dir, "SKILL.md", content)
        }
        InstallTarget::Cursor => {
            let dir = PathBuf::from(".cursor/skills/autoresearch");
            let content = templates::skill_md("cursor");
            (dir, "SKILL.md", content)
        }
        InstallTarget::Windsurf => {
            let dir = PathBuf::from(".windsurf/skills/autoresearch");
            let content = templates::skill_md("windsurf");
            (dir, "SKILL.md", content)
        }
        InstallTarget::Agents => {
            let dir = PathBuf::from(".agents/skills/autoresearch");
            let content = templates::skill_md("agents");
            (dir, "SKILL.md", content)
        }
        InstallTarget::All => unreachable!(),
    };

    let file_path = dir.join(filename);

    if file_path.exists() {
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
