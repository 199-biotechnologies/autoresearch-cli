# autoresearch

Universal autoresearch CLI — install skills, track experiments, view results across any AI coding agent.

Brings [Karpathy's autoresearch pattern](https://github.com/karpathy/autoresearch) to any project with any measurable metric.

## Install

```bash
cargo install autoresearch
```

Or from source:
```bash
git clone https://github.com/199-biotechnologies/autoresearch-cli
cd autoresearch-cli
cargo install --path .
```

## Quick Start

### 1. Install the skill into your agent

```bash
autoresearch install claude-code   # Claude Code
autoresearch install codex         # Codex CLI
autoresearch install opencode      # OpenCode
autoresearch install cursor        # Cursor
autoresearch install windsurf      # Windsurf
autoresearch install all           # All of the above
```

### 2. Initialize in your project

```bash
cd your-project
autoresearch init
```

This creates:
- `autoresearch.toml` — experiment configuration (target file, eval command, metric)
- `program.md` — research direction and ideas for the agent
- `.autoresearch/` — experiment logs

### 3. Edit program.md with your ideas

Tell the agent what to explore, what papers to reference, what constraints matter.

### 4. Start the loop

In your agent (Claude Code, Codex, etc.):
```
/autoresearch
```

Go to sleep. Wake up to results.

### 5. Check results

```bash
autoresearch status              # Overview
autoresearch log                 # Experiment history
autoresearch best                # Best result + diff
autoresearch diff 12 45          # Compare two experiments
autoresearch export --format csv # Export for analysis
```

## Commands

| Command | Description |
|---------|-------------|
| `install <target>` | Install skill into an AI agent |
| `init` | Initialize autoresearch in current project |
| `log [-n N]` | Show experiment history |
| `best` | Show best experiment + diff |
| `diff <a> <b>` | Compare two experiments |
| `status` | Project state and loop status |
| `export` | Export as CSV/JSON/JSONL |
| `agent-info` | Machine-readable capabilities |

All commands support `--json` for structured output (auto-enabled when piped).

## How It Works

The autoresearch pattern is simple:

1. You define: **one file** to modify, **one command** to evaluate, **one metric** to optimize
2. An AI agent runs a loop: modify the file → run eval → keep if better, revert if worse → repeat
3. Everything is tracked in git commits and `.autoresearch/experiments.jsonl`

This CLI handles the scaffolding and tracking. The agent handles the loop.

## Supported Agents

| Agent | Skill Format | Path |
|-------|-------------|------|
| Claude Code | SKILL.md | `~/.claude/skills/autoresearch/` |
| Codex CLI | SKILL.md | `~/.codex/skills/autoresearch/` |
| OpenCode | SKILL.md | `~/.config/opencode/skills/autoresearch/` |
| Cursor | .mdc rule | `.cursor/rules/autoresearch.mdc` |
| Windsurf | .md rule | `.windsurf/rules/autoresearch.md` |

## Configuration

`autoresearch.toml`:
```toml
target_file = "train.py"
eval_command = "python train.py"
metric_name = "val_bpb"
metric_direction = "lower"
time_budget = "5m"
branch = "autoresearch"
```

## Inspired By

- [Karpathy's autoresearch](https://github.com/karpathy/autoresearch) — the original pattern
- [uditgoenka/autoresearch](https://github.com/uditgoenka/autoresearch) — generalized Claude Code skill
- [ResearcherSkill](https://github.com/krzysztofdudek/ResearcherSkill) — domain-agnostic research agent
- [ARIS](https://github.com/wanshuiyin/Auto-claude-code-research-in-sleep) — cross-model research pipeline

## License

MIT
