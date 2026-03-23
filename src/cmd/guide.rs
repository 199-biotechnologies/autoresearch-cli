use crate::errors::CliError;
use crate::output::format::OutputFormat;
use crate::skill::templates;

pub fn run(json: bool) -> Result<(), CliError> {
    let format = OutputFormat::detect(json);
    let guide = templates::guide_text();

    match format {
        OutputFormat::Json => {
            let out = serde_json::json!({
                "status": "success",
                "data": {
                    "guide": guide,
                    "version": env!("CARGO_PKG_VERSION"),
                    "note": "This is the complete autoresearch methodology. It works with or without the skill installed. The skill auto-loads this into your agent's context; this command lets you read it manually.",
                }
            });
            println!("{}", serde_json::to_string_pretty(&out).unwrap());
        }
        OutputFormat::Table => {
            print!("{guide}");
        }
    }

    Ok(())
}
