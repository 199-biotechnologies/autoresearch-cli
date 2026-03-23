use crate::errors::CliError;
use crate::git;
use crate::output::format::OutputFormat;
use serde::Serialize;
use std::process::Command;

#[derive(Serialize)]
struct Check {
    name: String,
    passed: bool,
    message: String,
}

pub fn run(json: bool) -> Result<(), CliError> {
    let format = OutputFormat::detect(json);
    let mut checks: Vec<Check> = Vec::new();
    let mut all_passed = true;

    // 1. Git repo
    let git_ok = git::is_git_repo();
    checks.push(Check {
        name: "git_repo".into(),
        passed: git_ok,
        message: if git_ok {
            "Git repository found".into()
        } else {
            "Not a git repository. Run `git init`.".into()
        },
    });
    if !git_ok {
        all_passed = false;
    }

    // 2. autoresearch.toml exists
    let config_exists = std::path::Path::new("autoresearch.toml").exists();
    checks.push(Check {
        name: "config_file".into(),
        passed: config_exists,
        message: if config_exists {
            "autoresearch.toml found".into()
        } else {
            "No autoresearch.toml. Run `autoresearch init`.".into()
        },
    });
    if !config_exists {
        all_passed = false;
    }

    // 3. Parse config
    let config = if config_exists {
        match std::fs::read_to_string("autoresearch.toml") {
            Ok(content) => match toml::from_str::<toml::Table>(&content) {
                Ok(table) => {
                    checks.push(Check {
                        name: "config_valid".into(),
                        passed: true,
                        message: "autoresearch.toml parses correctly".into(),
                    });
                    Some(table)
                }
                Err(e) => {
                    checks.push(Check {
                        name: "config_valid".into(),
                        passed: false,
                        message: format!("Invalid TOML: {e}"),
                    });
                    all_passed = false;
                    None
                }
            },
            Err(e) => {
                checks.push(Check {
                    name: "config_valid".into(),
                    passed: false,
                    message: format!("Cannot read autoresearch.toml: {e}"),
                });
                all_passed = false;
                None
            }
        }
    } else {
        None
    };

    // 4. Required config fields
    if let Some(ref table) = config {
        for field in &[
            "target_file",
            "eval_command",
            "metric_name",
            "metric_direction",
        ] {
            let has_field = table
                .get(*field)
                .and_then(|v| v.as_str())
                .is_some_and(|s| !s.is_empty());
            checks.push(Check {
                name: format!("config_{field}"),
                passed: has_field,
                message: if has_field {
                    format!(
                        "{field} = {:?}",
                        table.get(*field).and_then(|v| v.as_str()).unwrap()
                    )
                } else {
                    format!("Missing required field: {field}")
                },
            });
            if !has_field {
                all_passed = false;
            }
        }
    }

    // 5. Target file exists
    if let Some(ref table) = config {
        if let Some(target) = table.get("target_file").and_then(|v| v.as_str()) {
            let exists = std::path::Path::new(target).exists();
            checks.push(Check {
                name: "target_file_exists".into(),
                passed: exists,
                message: if exists {
                    format!("{target} exists")
                } else {
                    format!("{target} not found — the agent won't have a file to modify")
                },
            });
            if !exists {
                all_passed = false;
            }
        }
    }

    // 6. Eval command runs
    if let Some(ref table) = config {
        if let Some(eval_cmd) = table.get("eval_command").and_then(|v| v.as_str()) {
            // Use perl for portable timeout (macOS doesn't have `timeout`)
            let result = Command::new("sh")
                .args(["-c", eval_cmd])
                .output();

            match result {
                Ok(output) if output.status.success() => {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    let has_number = stdout
                        .trim()
                        .lines()
                        .last()
                        .and_then(|line| {
                            line.split_whitespace()
                                .last()
                                .and_then(|w| w.parse::<f64>().ok())
                        })
                        .is_some();

                    checks.push(Check {
                        name: "eval_runs".into(),
                        passed: true,
                        message: format!(
                            "Eval command runs successfully (exit 0, output: {} bytes)",
                            stdout.len()
                        ),
                    });

                    checks.push(Check {
                        name: "eval_metric_parseable".into(),
                        passed: has_number,
                        message: if has_number {
                            "Eval output contains a parseable number".into()
                        } else {
                            format!(
                                "Could not find a number in eval output. Last line: {:?}",
                                stdout.trim().lines().last().unwrap_or("")
                            )
                        },
                    });
                    if !has_number {
                        all_passed = false;
                    }
                }
                Ok(output) => {
                    checks.push(Check {
                        name: "eval_runs".into(),
                        passed: false,
                        message: format!(
                            "Eval command failed (exit {}). stderr: {}",
                            output.status.code().unwrap_or(-1),
                            String::from_utf8_lossy(&output.stderr)
                                .chars()
                                .take(200)
                                .collect::<String>()
                        ),
                    });
                    all_passed = false;
                }
                Err(e) => {
                    checks.push(Check {
                        name: "eval_runs".into(),
                        passed: false,
                        message: format!("Cannot execute eval command: {e}"),
                    });
                    all_passed = false;
                }
            }
        }
    }

    // 7. Experiment branch
    if git_ok {
        if let Some(ref table) = config {
            let branch = table
                .get("branch")
                .and_then(|v| v.as_str())
                .unwrap_or("autoresearch");
            let exists = git::experiment_branch_exists(branch);
            checks.push(Check {
                name: "experiment_branch".into(),
                passed: true, // Not existing is OK — it'll be created
                message: if exists {
                    format!("Branch '{branch}' exists with experiments")
                } else {
                    format!("Branch '{branch}' will be created on first run")
                },
            });
        }
    }

    // 8. .autoresearch directory
    let log_dir = std::path::Path::new(".autoresearch").exists();
    checks.push(Check {
        name: "log_directory".into(),
        passed: true, // Will be created automatically
        message: if log_dir {
            ".autoresearch/ directory exists".into()
        } else {
            ".autoresearch/ will be created on first record".into()
        },
    });

    // 9. Stale lock file
    if git::is_loop_running() {
        checks.push(Check {
            name: "stale_lock".into(),
            passed: false,
            message: "Loop lock file exists (.autoresearch/loop.lock). If no loop is running, delete it.".into(),
        });
        all_passed = false;
    }

    // 10. program.md exists
    let program_exists = std::path::Path::new("program.md").exists();
    checks.push(Check {
        name: "program_md".into(),
        passed: program_exists,
        message: if program_exists {
            "program.md found — agent will read this for research direction".into()
        } else {
            "No program.md — agent will have no research direction guidance".into()
        },
    });
    if !program_exists {
        all_passed = false;
    }

    // 11. Git working tree clean
    if git_ok {
        let output = Command::new("git")
            .args(["status", "--porcelain"])
            .output()
            .ok();
        let clean = output
            .as_ref()
            .map(|o| o.stdout.is_empty())
            .unwrap_or(false);
        checks.push(Check {
            name: "git_clean".into(),
            passed: clean,
            message: if clean {
                "Working tree is clean".into()
            } else {
                "Uncommitted changes detected. Commit or stash before starting the loop.".into()
            },
        });
        // Not a blocker, just a warning
    }

    let passed_count = checks.iter().filter(|c| c.passed).count();
    let total_count = checks.len();

    match format {
        OutputFormat::Json => {
            let out = serde_json::json!({
                "status": if all_passed { "success" } else { "issues_found" },
                "data": {
                    "all_passed": all_passed,
                    "passed": passed_count,
                    "total": total_count,
                    "checks": checks,
                },
                "ready": all_passed,
            });
            println!("{}", serde_json::to_string_pretty(&out).unwrap());
        }
        OutputFormat::Table => {
            println!("Autoresearch Doctor\n");
            for check in &checks {
                let icon = if check.passed { "+" } else { "!" };
                println!("  [{icon}] {}: {}", check.name, check.message);
            }
            println!();
            println!(
                "{passed_count}/{total_count} checks passed. {}",
                if all_passed {
                    "Ready to start!"
                } else {
                    "Fix the issues above before starting the loop."
                }
            );
        }
    }

    Ok(())
}
