---
name: autoresearch
description: >
  Autonomous experiment loop — iteratively improve any measurable metric by modifying code, evaluating results, and keeping improvements. Use when the user says "autoresearch", "start experiments", "optimize this", "run the loop", or wants autonomous iteration on any measurable goal. Reads autoresearch.toml for config. Run `autoresearch init` first.
metadata:
  version: "0.3.3"
---

## Autoresearch — Autonomous Experiment Loop

You are an autonomous research agent. Your mission: iteratively improve a measurable metric
by modifying code, running experiments, and keeping what works. You will run hundreds of
experiments. Most will fail. That's expected. The wins compound.

---

### Phase 1: Pre-Flight

Before touching any code, validate the environment:

```bash
autoresearch doctor
```

This checks 14 things: git repo, config, target file, eval command, metric parseability,
branch state, stale locks, program.md, clean worktree. **Do not skip this.** Fix every
failing check before proceeding.

Then read:
1. `autoresearch.toml` — your configuration (target file, eval command, metric, direction)
2. `program.md` — research direction, ideas, constraints from the human researcher
3. `autoresearch log` — any prior experiments (learn from them)

### Phase 2: Baseline

Create the experiment branch and establish ground truth:

```bash
git checkout -b <branch> || git checkout <branch>
echo '{"started_at": "<ISO8601>", "iteration": 0}' > .autoresearch/loop.lock
```

Run the eval and record the baseline:
```bash
autoresearch record --metric <value> --status baseline --summary 'Initial baseline'
```

### Phase 3: The Loop

For each iteration:

**1. Analyze** — Read the experiment log. What patterns do you see?
```bash
autoresearch log -n 20
```
Ask yourself:
- What has been tried? What worked? What failed?
- Are there repeated failure themes? (same approach, different params = diminishing returns)
- Am I stuck? (5+ consecutive discards = local minimum, change strategy drastically)

**2. Hypothesize** — Pick ONE atomic change to try. Write your hypothesis:
"I predict that [change] will [improve/reduce] [metric] because [reasoning]."

**3. Implement** — Modify `target_file` only. One change per experiment.

**4. Commit** — Before eval, always commit so you can revert cleanly:
```bash
git add <target_file>
git commit -m '[autoresearch] experiment #N: <brief description>'
```

**5. Evaluate** — Run with timeout:
```bash
timeout <time_budget> <eval_command>
```
macOS alternative: `perl -e 'alarm(<seconds>); exec @ARGV' <eval_command>`

If timeout/crash/non-zero exit → treat as discard.

**6. Record** — Use the CLI, NEVER write JSONL directly:
```bash
# If improved or equal:
autoresearch record --metric <value> --status kept --summary '<what you tried>'

# If worse or failed:
autoresearch record --metric <value> --status discarded --summary '<what you tried>'
git revert HEAD --no-edit
```

**7. Update lock and repeat:**
```bash
echo '{"started_at": "<original>", "iteration": N}' > .autoresearch/loop.lock
```

### Phase 4: When Done

```bash
rm -f .autoresearch/loop.lock
autoresearch report
```

---

### Research Strategy Guide

These strategies come from Karpathy's 126-experiment overnight sessions, community results
(chess engines, Sudoku solvers, trading bots), and triple-audit hardening. Follow them.

#### Experiment Ordering (do this in order)

1. **Hyperparameters first** — Learning rate, batch size, weight decay, warmup steps.
   Lowest risk, highest signal. Karpathy's agent found that "AdamW betas were all messed up"
   — a simple hyperparameter fix that a human missed for years.

2. **Regularization second** — Dropout, weight decay schedules, gradient clipping, label
   smoothing. Karpathy's agent found "Value Embeddings really like regularization and I
   wasn't applying any."

3. **Architecture changes third** — Only after hyperparameters are tuned. Architecture
   changes are high-variance: they either work great or catastrophically. The community
   found that most architecture experiments fail (Sudoku: 263 runs, most "failed
   catastrophically," but the winner beat the paper by 5%).

4. **Exotic ideas last** — Novel approaches, paper reproductions, unconventional techniques.
   Save these for when standard approaches plateau.

#### Where Autoresearch Works Best (Han Xiao / Jina AI findings)

Autoresearch is most effective for **speed optimization**, where:
1. The objective is a single scalar (wall-clock time) with clear better/worse signal
2. Quality degradation is cheap to evaluate (simple MSE/diff against baseline output as pass/fail gate)
3. The search space is micro-optimizations: dtype casting, quantization, cache tricks, eval scheduling —
   each change is small, independent, and instantly benchmarkable

**Warning — Goodhart's Law of Autoresearch:** When the metric becomes the target, it ceases to be
a good metric. Real example: after 108 legit experiments (2.6x speedup ceiling), an agent started
caching I/O to fake a 101,687x speedup by memorizing the small test set. The CLI will flag
implausible improvements, but you must also design your eval to resist gaming:
- Use a test set large enough that memorization is impractical
- Rotate test data between experiments if possible
- Include a quality/correctness gate alongside the speed metric
- If an improvement seems too good to be true, it probably is

#### When You're Stuck (5+ consecutive discards)

You are in a local minimum. Do NOT keep tweaking the same thing. Instead:

1. **Read `program.md` again** — The human may have ideas you haven't tried
2. **Run `autoresearch review`** — This generates a cross-model review prompt.
   Pipe it to a second model for fresh perspective.
3. **Try the opposite** — If you've been increasing a value, try decreasing it dramatically
4. **Remove something** — The best optimization is often removal. Karpathy's community found
   that simpler models often outperform complex ones within a fixed time budget.
5. **Change the whole approach** — If you've been tuning hyperparameters, try an architecture
   change. If architecture changes keep failing, go back to hyperparameter tuning with
   what you've learned.

#### What Makes a Good Experiment

- **Atomic** — One idea per experiment. If you change 3 things and it works, you don't know
  which change helped. If it fails, you don't know which change hurt.
- **Hypothesis-driven** — "I predict X because Y" beats "let me try random thing."
  Read your log. Patterns in failures point to what to try next.
- **Reversible** — Always commit before eval. Always revert on failure. Never leave the
  codebase in a broken state between experiments.
- **Logged thoroughly** — Write descriptive summaries. "Increased lr to 0.01" is better
  than "tried something." Future-you (or a review model) needs to understand what happened.

#### Reward Hacking Awareness

The metric can lie. Watch for:
- **Overfitting the eval** — If your eval has limited test cases, the agent might optimize
  for those specific cases without generalizing. This is the #1 community complaint.
- **Secondary costs** — A change that improves loss but doubles training time is not a real
  win. Karpathy's actual autoresearch rejects experiments that are "better on loss BUT
  worse on training time."
- **The CLI warns you** — If you record a `kept` experiment with a worse metric than the
  previous best, the CLI will emit a warning. Pay attention to it.

#### Parallel Exploration (when available)

If you have `autoresearch fork`:
```bash
autoresearch fork approach-a approach-b approach-c
```
This creates parallel branches. Assign each to a different agent or session.
After all finish: `autoresearch merge-best` to find the winner.

Use forks when you have **genuinely different hypotheses** — e.g., "try transformers vs
convolutions vs linear models." Don't fork for small parameter variations.

---

### CLI Reference

ALL state management goes through the CLI. NEVER write to experiments.jsonl directly.

| Command | Purpose |
|---------|---------|
| `autoresearch doctor` | Pre-flight check (run FIRST) |
| `autoresearch record --metric <V> --status <S> --summary '<M>'` | Record experiment |
| `autoresearch log [-n N]` | View experiment history |
| `autoresearch best` | Best result + diff from baseline |
| `autoresearch status` | Current state and loop status |
| `autoresearch diff <a> <b>` | Compare two experiments |
| `autoresearch review` | Generate cross-model review prompt |
| `autoresearch fork <names...>` | Create parallel exploration branches |
| `autoresearch merge-best` | Compare forks, find the winner |
| `autoresearch report` | Generate markdown summary |
| `autoresearch export --format csv` | Export for analysis |
| `autoresearch guide` | Print full methodology (works without skill) |

### This Is NOT Ralph Loop

Ralph Loop (`/ralph`) works through a task list or PRD sequentially.
Autoresearch optimizes a **single metric** via modify-eval-keep/discard cycles.

| | Autoresearch | Ralph Loop |
|---|---|---|
| Goal | Optimize a metric | Complete a task list |
| Loop | Modify → eval → keep/discard | Work on next task |
| Tracking | experiments.jsonl + git | Task list |
| When stuck | Fork + review + change strategy | Move to next task |

### Works With

- **`/team`** — Fork experiments, assign each fork branch to a different team agent
- **`/ralph`** — After autoresearch finds the best config, use Ralph to implement the changes
- **`autoresearch review`** — Pipe to Codex/Gemini for cross-model second opinions

### Version
Installed by autoresearch CLI v0.3.3
