# Milestone report — template

The controller fills this in as the body of the milestone pull request (milestone branch into `master`), after the
final review and the checks of [workflow.md](workflow.md#the-end-of-a-milestone). The wording is public: no account,
workspace, host or person names, no emails or ids, nothing from the private lists, no security finding. Run the
privacy check on the text before posting it ([workflow.md](workflow.md#docs)). Each part has one line of guidance in
italics; replace the placeholder.

```markdown
# Milestone <name>: <one line on what it holds>

## 1. Calls made for you

_Every decision the controller or an implementer made that the maintainer might have made differently, one line each,
with why._

- <the call> — <why>

## 2. Things to try yourself

_A short hands-on list for the changes you will notice._

- <what to do, what you should see>

## 3. Per issue

_What changed, the merged unit pull request, what was checked LIVE, UNIT or NOT, and what was not tried._

- #<N> <title> — <what changed>. Pull request #<M>. LIVE: <…>. UNIT: <…>. NOT: <…>.

## 4. Reviews

_What the reviews found and fixed, what was dropped as a nit (why), what was filed as an issue._

- Fixed: <…>. Dropped: <…> (<why>). Filed: #<N> (<why>).

## 5. Health

_The tests, the Windows build, CI, the final live check._

- `cargo test`: <result>. Windows: <build and `--no-run` result>. CI: <run and result>. Live smoke check: <result>.

## 6. Refused actions

_Each one was raised at once. What it was, how it was settled, the allowlist or recipe change it led to. "None" if
there were none._

- <the command> — <how it was settled> — <the change>

## 7. A build to try

_Where a debug build of the milestone branch is (a scratch `--target-dir`, never `target/release`) and the one command
to run it._

    <one command>

Reply "merge" to merge this pull request with a merge commit, and say whether I may delete the milestone's branches on
GitHub.
```
