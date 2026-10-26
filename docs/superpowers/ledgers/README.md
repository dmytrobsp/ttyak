# Plan ledgers

A ledger is a plan's record of what happened while it ran: the state of every task, what was checked and how, the
controller's rulings and what was deferred. It is how work resumes after a crash, a lost session or a new machine.
The workflow around it is in [workflow.md](../../agents/workflow.md#executing-a-plan-with-subagents).

## Where

- **Committed**: `docs/superpowers/ledgers/<plan>.md`, named like its plan (`2026-11-02-<topic>.md` for
  `docs/superpowers/plans/2026-11-02-<topic>.md`). The controller commits it at every task boundary, in its own
  docs-only commit (no CI runs).
- **Not committed**: the bulky scratch — the rules file given to every implementer, task briefs, review packages,
  implementers' reports — in `.superpowers/sdd/<plan>/`, git-ignored. Never `git add -f` it: it can hold private names.

Plans started before 2026-10-27 kept their ledgers in the scratch folder only; this convention applies from the next
plan on. Their outcomes are in [docs/history/](../../history/README.md) and `git log`.

## What goes in

The head says which plan and design it belongs to and the commit the plan started from. Then one entry per event, in
order, short lines:

- **Task status**: dispatched (with the base commit), its commits, review rounds, complete (the commit range).
- **What was checked, per task**: `LIVE` (exercised in the real app against the test workspace, and checked on screen
  or by reading back as the other account), `UNIT` (tests only), `NOT` (written, not exercised — say why). Name what
  was not tried; never round it up.
- **Rulings**: a decision the controller made without the maintainer, with what it costs if wrong.
- **The maintainer's decisions**, with the date and their words where they matter.
- **Deferred items**: what a review left for later, and the issue filed for it (`#N`) once filed.
- **Outside events**: a CI run that went red and its fix, a refused action, a machine that was busy.

Nothing private: no workspace, account, machine or person names, no emails or ids (`AGENTS.local.md` holds those).
Wording is public, as in commits and issues.

## Example

```markdown
# Ledger — plan docs/superpowers/plans/2026-11-02-example.md

Design: docs/superpowers/specs/2026-11-02-example-design.md. Start: 1a2b3c4.

- Task 1: dispatched (base 1a2b3c4)
- Task 1: 5d6e7f8; review: 2 minors fixed in 9a0b1c2; complete (1a2b3c4..9a0b1c2).
  LIVE: the new row in F6, toggled as A, read back as B. UNIT: settings round-trip. NOT: macOS (no Mac session).
- Ruling: kept the old key as an alias — if wrong, one line to drop.
- Deferred: the help page wording → #70.
- Task 2: dispatched (base 9a0b1c2)
```

When the plan ends, the ledger stays as it is: [docs/PROGRESS.md](../../PROGRESS.md) points at the plan only while it
is in flight.
