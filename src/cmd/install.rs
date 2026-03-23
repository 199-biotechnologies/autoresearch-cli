use crate::cli::InstallTarget;
use crate::errors::CliError;
use crate::output::format::OutputFormat;
use crate::skill;

pub fn run(target: &InstallTarget, json: bool) -> Result<(), CliError> {
    let format = OutputFormat::detect(json);

    let result = skill::install(target);

    match result {
        Ok(installed) => match format {
            OutputFormat::Json => {
                let out = serde_json::json!({
                    "status": "success",
                    "installed": installed,
                });
                println!("{}", serde_json::to_string_pretty(&out).unwrap());
            }
            OutputFormat::Table => {
                println!("Installed autoresearch skill:");
                for path in &installed {
                    println!("  -> {path}");
                }
                println!();
                println!(
                    "Next: run `autoresearch init` in your project to set up experiments."
                );
            }
        },
        Err(CliError::AlreadyInstalled(path)) => match format {
            OutputFormat::Json => {
                let out = serde_json::json!({
                    "status": "already_installed",
                    "message": format!("Skill already installed at {path}"),
                    "path": path,
                    "suggestion": "Use `autoresearch install <target>` after updating the CLI to reinstall with the latest skill version."
                });
                println!("{}", serde_json::to_string_pretty(&out).unwrap());
            }
            OutputFormat::Table => {
                println!("Skill already installed at {path}");
                println!("Reinstall after CLI update to get the latest skill version.");
            }
        },
        Err(e) => return Err(e),
    }

    Ok(())
}
