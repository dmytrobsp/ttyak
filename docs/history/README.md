# History

The status table of the old root `PROGRESS.md` and its bug log, kept as they were. Only the line breaks changed: each
row is now a section, wrapped at about 120 columns. Private names were replaced (see below).

    python3 tools/docs/split_history.py PROGRESS.md docs/history --check --redact <map>

compares the whitespace-normalised text of this archive with the old rows and bug log (after the same replacement) and
fails on any difference. The old `PROGRESS.md` is in git history (`git show 3f2e532:PROGRESS.md` writes the last
version to give the check).

The archive was redacted with the maintainer's private map, which is not in the repository, so the check can only be
re-run where that map exists. The map's `__keep__` list names the public addresses that are left alone
(`github.com/<owner>/ttyak`, `<owner>.github.io`, the winget id).

| File | Rows | Era |
|---|---|---|
| [`0.1-0.9.md`](0.1-0.9.md) | the first rows (no ids) up to `VY-final 2 fix`: A–Z, LP (landing page), AA, VW, VX, VY | slacktui and klack, up to v0.9.0 and the queues after it |
| [`0.10-0.11.md`](0.10-0.11.md) | `TY0`–`TY8`, `D1`–`D24` | the rename to ttyak (0.10.0) and distribution (0.11.0) |
| [`0.12.md`](0.12.md) | `AT1`–`AT19`, `RW1`–`RW13`, `RW-final`, `PT1`–`PT11` | attention (0.12.0), the live-check fixes (0.12.1), the README/wiki split and 0.12.2 |
| [`bug-log.md`](bug-log.md) | the whole bug log | all eras |

The ids repeat between eras (`D1` in `0.1-0.9.md` is a search fix; `D1` in `0.10-0.11.md` is distribution step 1).
Rows are in the order they stood in the table, which is the order they were done.

## How to read a row

The heading is the row's first sentence (usually `<id> — <title>`), the text under it is the rest of the first cell,
then the three bullets are the table's other cells:

- **State**: `done`, or what was left.
- **Verified**: what was checked, and how.
  - `LIVE`: exercised in a real kitty window against a test workspace and checked by screenshot or by reading back as
    the other account.
  - `UNIT`: covered by `cargo test` (or the tool's own tests) only.
  - `NOT`: written down as not checked, with the reason.
- **Commit**: the commit hash or hashes.

The names of the test workspace, its channels and its people were replaced with neutral words ("the test workspace",
`channel-1`, a made-up first name, `a-handle`). Public addresses such as `github.com/<owner>/ttyak` were left alone.
The wording is otherwise the original; the tool's `--redact` option did the replacing, from a map kept outside the
repository.

The first rows of `0.1-0.9.md` have no id and a short heading; they are the area names of the original table. Ten
rows did not split into four cells (a `|` in the text); they are kept whole, with no bullets. The one line under the
table (the test counts at 0.12.2) is the last section of `0.12.md`.

The old "Next" paragraphs were not carried over: what was still open in them went to GitHub Issues
(see the project's issue queue).
