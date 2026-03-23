/// The canonical autoresearch skill content — shared core instructions
fn core_skill_body() -> String {
    format!(
        r##"
## Autoresearch Loop — Autonomous Experiment Iteration

You are an autonomous research agent. Your job is to iteratively improve a measurable metric
by modifying code, running experiments, and keeping what works.

### Setup (run once at start)

1. Read `autoresearch.toml` for configuration:
   - `target_file` — the single file you may modify
   - `eval_command` — the command that produces the metric
   - `metric_name` — what the metric is called (e.g., val_bpb, accuracy, p99_latency)
   - `metric_direction` — "lower" or "higher" (which direction is better)
   - `time_budget` — max time per experiment (e.g., "5m")
   - `branch` — git branch for experiments (default: autoresearch)

2. Read `program.md` for research direction and ideas

3. Create git branch if it doesn't exist:
   ```bash
   git checkout -b <branch> || git checkout <branch>
   ```

4. Create a lock file to signal the loop is running:
   ```bash
   echo '{{"started_at": "<ISO8601>", "iteration": 0}}' > .autoresearch/loop.lock
   ```

5. Run baseline evaluation and record it:
   ```bash
   METRIC=$(<eval_command>)
   autoresearch record --metric $METRIC --status baseline --summary "Initial baseline"
   ```

### The Loop (repeat indefinitely)

For each iteration N:

1. **Plan** — Based on previous results, `program.md`, and the target file, decide ONE atomic change to try.
   Run `autoresearch log` to see what's been tried. Don't repeat failed approaches.

2. **Implement** — Make the change to `target_file` only. Keep changes atomic and small.

3. **Commit before eval** — So we can revert cleanly:
   ```bash
   git add <target_file>
   git commit -m "[autoresearch] experiment #N: <brief description>"
   ```

4. **Evaluate** — Run the eval command with the time budget:
   ```bash
   timeout <time_budget> <eval_command>
   ```
   If the command times out or returns non-zero, treat as a failed experiment (discard).
   If the metric cannot be parsed from output, treat as failed (discard).

5. **Decide and Record** — Parse the metric from eval output:
   - If metric improved (or equal): **KEEP**
     ```bash
     autoresearch record --metric <value> --status kept --summary "<what you tried>"
     ```
   - If metric worsened or eval failed: **DISCARD** and revert:
     ```bash
     autoresearch record --metric <value> --status discarded --summary "<what you tried>"
     git revert HEAD --no-edit
     ```

6. **Update lock** — Update the iteration count:
   ```bash
   echo '{{"started_at": "<original>", "iteration": N}}' > .autoresearch/loop.lock
   ```

7. **Repeat** — Go to step 1. Never stop unless interrupted.

### When Done

Remove the lock file:
```bash
rm -f .autoresearch/loop.lock
```

### Rules

- **One file only** — Only modify `target_file`. Everything else is read-only.
- **Atomic changes** — One idea per experiment. Don't combine multiple changes.
- **Commit before eval** — Always commit before running the eval so you can revert cleanly.
- **Mechanical verification only** — The eval command is the only judge. Don't use your own judgment about whether a change "should" help.
- **JSONL is canonical state** — The `.autoresearch/experiments.jsonl` file (managed by the CLI) is the source of truth. Git commits are secondary. NEVER write to the JSONL file directly.
- **Eval must print exactly one number** — The eval command must print the metric value as its last line of stdout. If it prints multiple lines, only the last number matters.
- **Log everything** — Even discarded experiments are valuable data. Record them.
- **Read the log** — Before each experiment, run `autoresearch log` to see what's been tried. Don't repeat failed approaches unless you have a new angle.
- **Creative when stuck** — If the last 5 experiments were all discarded, try a completely different approach. Read `program.md` for inspiration. Consider running `autoresearch review` for cross-model analysis.
- **Simplicity wins** — Prefer the simplest change that improves the metric.
- **Multi-objective awareness** — If your eval measures time/cost alongside the primary metric, a change that improves the metric but doubles runtime may not be a real win. Consider secondary effects.
- **macOS timeout** — If `timeout` is not available, use `perl -e 'alarm(<seconds>); exec @ARGV' <eval_command>` as a portable alternative.

### CLI Integration

Use the `autoresearch` CLI for ALL state management. NEVER write to experiments.jsonl directly.
- `autoresearch record --metric <V> --status <S> --summary '<M>'` — record experiment result (use single quotes around summary to avoid shell escaping issues)
- `autoresearch status` — check current state and best metric
- `autoresearch log` — view experiment history
- `autoresearch best` — see the best result so far
- `autoresearch diff <a> <b>` — compare two experiments

### Version
Installed by autoresearch CLI v{version}
"##,
        version = env!("CARGO_PKG_VERSION")
    )
}

pub fn claude_code_skill() -> String {
    format!(
        r#"---
name: autoresearch
description: >
  Autonomous experiment loop — iteratively improve any measurable metric by modifying code,
  evaluating results, and keeping improvements. Use when the user says "autoresearch",
  "start experiments", "optimize this", "run the loop", or wants autonomous iteration on
  any measurable goal. Reads autoresearch.toml for config. Run `autoresearch init` first.
version: {version}
---
{body}"#,
        version = env!("CARGO_PKG_VERSION"),
        body = core_skill_body()
    )
}

pub fn codex_skill() -> String {
    format!(
        r#"---
name: autoresearch
description: >
  Autonomous experiment loop — iteratively improve any measurable metric by modifying code,
  evaluating results, and keeping improvements. Use when the user says "autoresearch",
  "start experiments", "optimize this", or wants autonomous iteration. Reads autoresearch.toml.
version: {version}
---
{body}"#,
        version = env!("CARGO_PKG_VERSION"),
        body = core_skill_body()
    )
}

pub fn opencode_skill() -> String {
    format!(
        r#"---
name: autoresearch
description: >
  Autonomous experiment loop — iteratively improve any measurable metric by modifying code,
  evaluating results, and keeping improvements. Reads autoresearch.toml for configuration.
version: {version}
---
{body}"#,
        version = env!("CARGO_PKG_VERSION"),
        body = core_skill_body()
    )
}

pub fn cursor_rule() -> String {
    format!(
        r#"---
description: >
  Autoresearch autonomous experiment loop — activate when user says "autoresearch",
  "start experiments", "optimize this", or wants iterative metric improvement.
  Reads autoresearch.toml for config.
globs: "**/autoresearch.toml"
alwaysApply: false
---
{body}"#,
        body = core_skill_body()
    )
}

pub fn windsurf_rule() -> String {
    format!(
        r#"---
trigger: glob
glob: "**/autoresearch.toml"
---
{body}"#,
        body = core_skill_body()
    )
}
