use crate::cli::InstallTarget;
use crate::errors::CliError;
use crate::output::format::OutputFormat;
use crate::skill;

pub fn run(target: &InstallTarget, json: bool) -> Result<(), CliError> {
    let format = OutputFormat::detect(json);
    let installed = skill::install(target)?;

    match format {
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
            println!("Next: run `autoresearch init` in your project to set up experiments.");
        }
    }

    Ok(())
}
