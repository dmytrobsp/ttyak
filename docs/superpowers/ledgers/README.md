# Plan ledgers

A ledger is a plan's record of what happened while it ran: the state of every task, what was checked and how, the
controller's rulings and what was deferred. It is how work resumes after a crash, a lost session or a new machine.
The workflow around it is in [workflow.md](../../agents/workflow.md#executing-a-plan-with-subagents).

## Where

- **Committed**: `docs/superpowers/ledgers/<plan>.md`, named like its plan (`2026-11-02-<topic>.md` for
  `docs/superpowers/plans/2026-11-02-<topic>.md`). The controller commits its head (plan, design, start commit,
  the task list) as soon as the plan starts — so a plan in flight always has a committed ledger — then again at every
  task boundary, each time in its own docs-only commit (no CI runs).
- **Not committed**: the bulky scratch — the rules file given to every implementer, task briefs, review packages,
  implementers' reports — in `.superpowers/sdd/<plan>/`, git-ignored. Never `git add -f` it: it can hold private
  names.

Plans started before 2026-10-27 — the documentation housekeeping plan of 2026-10-26 included — kept their ledgers
git-ignored in the scratch folder, so no committed ledger exists for them; their outcomes are in
[docs/history/](../../history/README.md) (up to 0.12.2) and `git log`. The committed convention starts with the next
plan.

## What goes in

The head says which plan and design it belongs to and the commit the plan started from, and lists its tasks. Then one
entry per event, in order, short lines:

- **Task status**: dispatched (with the base commit), its commits, review rounds, complete (the commit range).
- **What was checked, per task**: `LIVE` (exercised in the real app against the test workspace, and checked on screen
  or by reading back as the other account), `UNIT` (tests only), `NOT` (written, not exercised — say why). Name what
  was not tried; never round it up.
- **Rulings**: a decision the controller made without the maintainer, with what it costs if wrong.
- **The maintainer's decisions**, with the date and their words where they matter.
- **Findings not fixed**: a nit dropped (and why), or an item filed as an issue because it is outside the plan's
  scope or needs the maintainer's decision (`#N`). Every other review finding is fixed before the plan ends
  ([workflow](../../agents/workflow.md#executing-a-plan-with-subagents)).
- **Outside events**: a CI run that went red and its fix, a refused action, a machine that was busy.

Nothing private: no workspace, account, machine or person names, no emails or ids (`AGENTS.local.md` holds those).
Wording is public, as in commits and issues.

## A milestone's ledger

A milestone run with parallel implementers ([workflow.md](../../agents/workflow.md#running-a-milestone)) has one
ledger, committed on `master` like any other (not on the milestone branch), with these lines besides:

- **The start sitting**: the maintainer's answers, with the date, and the batching (which units run at once, which
  wait, and why).
- **Per unit**: its branch (`m/<milestone>/<unit>`), pull request, dispatch time, approval time and merge time, the
  review rounds, and the gate results (merge conflicts, red runs, and what they say about the batching).
- **Every refused action**: the command, what it was needed for, how it was settled, and the allowlist or recipe
  change it led to.
- **At the end**: the final review's outcome, the checks, and the retro.

After a lost session, this ledger, `git log`, the open pull requests, the worktrees on disk
(`python3 tools/milestone/milestone.py ls`) and the agent ids in `.superpowers/sdd/<plan>/agents.md` (unit, role,
id: git-ignored, so the agents can be resumed) are how the controller resumes.

## Example

```markdown
# Ledger — plan docs/superpowers/plans/2026-11-02-example.md

Design: docs/superpowers/specs/2026-11-02-example-design.md. Start: 1a2b3c4.
Tasks: 1 the F6 row, 2 the help page, 3 final review.

- Task 1: dispatched (base 1a2b3c4)
- Task 1: 5d6e7f8; review: 2 minors fixed in 9a0b1c2; complete (1a2b3c4..9a0b1c2).
  LIVE: the new row in F6, toggled as A, read back as B. UNIT: settings round-trip. NOT: macOS (no Mac session).
- Ruling: kept the old key as an alias — if wrong, one line to drop.
- Dropped (nit): a double space in a code comment. Filed: the help page's wording needs the maintainer's call → #70.
- Task 2: dispatched (base 9a0b1c2)
```

When the plan ends, a last entry says so ("Plan complete", with the final review's outcome) and the ledger stays as
it is: [docs/PROGRESS.md](../../PROGRESS.md) points at the plan only while it is in flight.
