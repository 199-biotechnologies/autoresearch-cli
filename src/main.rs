mod cli;
mod cmd;
mod errors;
mod git;
mod output;
mod skill;

use std::process;

fn main() {
    let cli = cli::parse();
    let json_flag = cli.json;
    if let Err(e) = cmd::run(cli) {
        let format = output::format::OutputFormat::detect(json_flag);
        match format {
            output::format::OutputFormat::Json => {
                let err_json = serde_json::json!({
                    "status": "error",
                    "error": {
                        "code": e.error_code(),
                        "message": e.to_string(),
                        "suggestion": e.suggestion(),
                    }
                });
                println!("{}", serde_json::to_string_pretty(&err_json).unwrap());
            }
            output::format::OutputFormat::Table => {
                eprintln!("error: {e}");
            }
        }
        process::exit(e.exit_code());
    }
}
