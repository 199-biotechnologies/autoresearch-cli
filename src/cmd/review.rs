use crate::errors::CliError;
use crate::git;
use crate::output::format::OutputFormat;

pub fn run(json: bool) -> Result<(), CliError> {
    let format = OutputFormat::detect(json);

    let config = load_config()?;
    let branch = config
        .get("branch")
        .and_then(|v| v.as_str())
        .unwrap_or("autoresearch");
    let metric_name = config
        .get("metric_name")
        .and_then(|v| v.as_str())
        .unwrap_or("metric");
    let metric_direction = config
        .get("metric_direction")
        .and_then(|v| v.as_str())
        .unwrap_or("lower");
    let target_file = config
        .get("target_file")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    let experiments = git::parse_experiments(branch, 10000)?;
    if experiments.is_empty() {
        return Err(CliError::NoExperiments(branch.to_string()));
    }

    let total = experiments.len();
    let kept: Vec<_> = experiments
        .iter()
        .filter(|e| e.status == git::ExperimentStatus::Kept)
        .collect();
    let discarded: Vec<_> = experiments
        .iter()
        .filter(|e| e.status == git::ExperimentStatus::Discarded)
        .collect();
    let baseline = experiments
        .iter()
        .find(|e| e.status == git::ExperimentStatus::Baseline);

    let lower_is_better = metric_direction != "higher";
    let best = experiments
        .iter()
        .filter(|e| {
            e.metric.is_some()
                && (e.status == git::ExperimentStatus::Kept
                    || e.status == git::ExperimentStatus::Baseline)
        })
        .min_by(|a, b| {
            let ma = a.metric.unwrap();
            let mb = b.metric.unwrap();
            if lower_is_better {
                crate::git::safe_cmp(ma, mb)
            } else {
                crate::git::safe_cmp(mb, ma)
            }
        });

    // Detect patterns
    let recent_discards = experiments
        .iter()
        .take(10)
        .filter(|e| e.status == git::ExperimentStatus::Discarded)
        .count();
    let stuck = recent_discards >= 7;

    let discard_rate = if total > 0 {
        (discarded.len() as f64 / total as f64) * 100.0
    } else {
        0.0
    };

    // Find repeated themes in discarded experiments
    let discard_summaries: Vec<&str> = discarded.iter().map(|e| e.summary.as_str()).collect();

    // Build the review prompt for a second model
    let mut review_prompt = String::new();
    review_prompt.push_str("# Autoresearch Experiment Review Request\n\n");
    review_prompt.push_str("You are reviewing an autoresearch experiment session. ");
    review_prompt.push_str("Your job is to identify patterns, suggest new directions, ");
    review_prompt.push_str("and flag potential issues the primary agent may have missed.\n\n");

    review_prompt.push_str("## Session Summary\n\n");
    review_prompt.push_str(&format!("- **Target file:** `{target_file}`\n"));
    review_prompt.push_str(&format!(
        "- **Metric:** {metric_name} ({metric_direction} is better)\n"
    ));
    review_prompt.push_str(&format!("- **Total experiments:** {total}\n"));
    review_prompt.push_str(&format!(
        "- **Kept:** {} ({:.0}%)\n",
        kept.len(),
        100.0 - discard_rate
    ));
    review_prompt.push_str(&format!(
        "- **Discarded:** {} ({:.0}%)\n",
        discarded.len(),
        discard_rate
    ));
    if let Some(bl) = baseline {
        review_prompt.push_str(&format!(
            "- **Baseline:** {}\n",
            bl.metric.map(|m| format!("{:.6}", m)).unwrap_or_default()
        ));
    }
    if let Some(b) = best {
        review_prompt.push_str(&format!(
            "- **Best so far:** {} (run #{})\n",
            b.metric.map(|m| format!("{:.6}", m)).unwrap_or_default(),
            b.run
        ));
    }

    review_prompt.push_str("\n## Winning Changes (what worked)\n\n");
    for exp in &kept {
        review_prompt.push_str(&format!(
            "- Run #{}: {} → {}\n",
            exp.run,
            exp.metric.map(|m| format!("{:.6}", m)).unwrap_or_default(),
            exp.summary
        ));
    }

    review_prompt.push_str("\n## Failed Attempts (what didn't work)\n\n");
    for exp in discarded.iter().take(20) {
        review_prompt.push_str(&format!(
            "- Run #{}: {} → {}\n",
            exp.run,
            exp.metric.map(|m| format!("{:.6}", m)).unwrap_or_default(),
            exp.summary
        ));
    }
    if discarded.len() > 20 {
        review_prompt.push_str(&format!("- ... and {} more\n", discarded.len() - 20));
    }

    review_prompt.push_str("\n## Questions for Review\n\n");
    review_prompt.push_str("1. What patterns do you see in what worked vs what didn't?\n");
    review_prompt.push_str("2. Are there unexplored directions the agent should try?\n");
    review_prompt.push_str(
        "3. Is the agent stuck in a local minimum? If so, what radical change might help?\n",
    );
    review_prompt.push_str("4. Are any of the kept changes suspicious (might be noise)?\n");
    review_prompt
        .push_str("5. What would you try next if you were running the next 20 experiments?\n");

    if stuck {
        review_prompt.push_str("\n## WARNING: Agent appears stuck\n\n");
        review_prompt.push_str(&format!(
            "The last 10 experiments had {recent_discards} discards. The agent may be stuck.\n"
        ));
        review_prompt.push_str("Please suggest a fundamentally different approach.\n");
    }

    // Detect repeated failure patterns
    if !discard_summaries.is_empty() {
        let mut word_freq: std::collections::HashMap<&str, usize> =
            std::collections::HashMap::new();
        for summary in &discard_summaries {
            for word in summary.split_whitespace() {
                let w = word.trim_matches(|c: char| !c.is_alphanumeric());
                if w.len() > 3 {
                    *word_freq.entry(w).or_insert(0) += 1;
                }
            }
        }
        let mut frequent: Vec<_> = word_freq.into_iter().filter(|(_, c)| *c >= 3).collect();
        frequent.sort_by(|a, b| b.1.cmp(&a.1));
        if !frequent.is_empty() {
            review_prompt.push_str("\n## Repeated Themes in Failed Experiments\n\n");
            for (word, count) in frequent.iter().take(10) {
                review_prompt.push_str(&format!("- \"{word}\" appeared in {count} failures\n"));
            }
        }
    }

    match format {
        OutputFormat::Json => {
            let out = serde_json::json!({
                "status": "success",
                "data": {
                    "review_prompt": review_prompt,
                    "summary": {
                        "total": total,
                        "kept": kept.len(),
                        "discarded": discarded.len(),
                        "discard_rate_pct": discard_rate,
                        "stuck": stuck,
                        "baseline_metric": baseline.and_then(|b| b.metric),
                        "best_metric": best.and_then(|b| b.metric),
                    }
                },
                "suggestion": "Pipe this prompt to a second model: autoresearch review --json | jq -r '.data.review_prompt' | codex exec -m gpt-5.4 --full-auto"
            });
            println!("{}", serde_json::to_string_pretty(&out).unwrap());
        }
        OutputFormat::Table => {
            println!("{review_prompt}");
            println!("---");
            println!("Copy the above to a second model (Codex, Gemini) for cross-model review.");
            println!("Or pipe it: autoresearch review | codex exec --full-auto");
        }
    }

    Ok(())
}

fn load_config() -> Result<toml::Table, CliError> {
    let path = std::path::Path::new("autoresearch.toml");
    if !path.exists() {
        return Err(CliError::Config(
            "No autoresearch.toml found. Run `autoresearch init` first.".into(),
        ));
    }
    let content = std::fs::read_to_string(path)?;
    toml::from_str(&content).map_err(|e| CliError::Config(e.to_string()))
}
