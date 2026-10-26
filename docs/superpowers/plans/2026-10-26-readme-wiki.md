# README → short README + GitHub wiki — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or
> superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Turn the 825-line README into a short front door plus a checked, published GitHub wiki catalogue, and bring
the site, the packaging and the code strings that point at the README in line.

**Architecture:** The wiki's pages are written in the main repo under `docs/wiki/` (GitHub-wiki file naming) and
published to `dmytrobsp/ttyak.wiki.git` by a script that first runs a link checker and the existing privacy scan.
Content moves section by section, each checked against the code; the README is cut only after the wiki is published,
then the site, packaging and code strings are re-pointed.

**Tech Stack:** Markdown (GitHub wiki + README), Python 3 stdlib (`unittest`) for the tools, bash for the publish
script, Rust for two code strings and one test, the existing site tooling (`tools/site/`).

**Spec:** `docs/superpowers/specs/2026-10-26-readme-wiki-design.md` — read it before any task; it holds the why, the
README outline, the catalogue, the accuracy-check rule, the screenshot rules and the list of README dependants.

## Global Constraints

- The Bash tool runs bash, not fish: `NAME=value`, every multi-command script starts with `set -euo pipefail`, never a
  recursive chmod/chown/rm/mv/cp (or `find -delete`) on a path built from a variable (literal path or `${VAR:?}` +
  a check). Put these rules into every subagent prompt.
- No `cargo fmt`; never build `target/release`; `cargo build` before any live run.
- Small verified steps, one commit each; `git commit` with explicit paths, never chained with a scripted edit; never
  `git add -f .superpowers`; trailer `Co-Authored-By: Claude <model> <noreply@anthropic.com>` naming the model.
- Routine pushes of master need no asking; tags/releases/force-push do. **No release in this project.**
- Docs-only commits (`**.md`, `docs/**`, `site/**`, `tools/site/**`) skip CI; any push touching `crates/`, `tools/`
  (outside `tools/site/`), `packaging/` or `.github/workflows/` runs CI: watch it (one capped waiter, `gh run watch`).
  A push touching `site/**`, `tools/site/**` or the paths in `pages.yml` deploys the site: watch the Pages run.
- **Publishing to the wiki is public**: the user sees the drafts and okays every publish during this project.
- **The public wiki describes the latest release** (the user). The first publish is from master (no code changes since
  v0.12.1, checked).
- Public text and images: no test-account names, Slack ids, emails, workspace names of the test or work workspaces, or
  machine details. Screenshots: workspace name **blurred**, every person a **made-up name**, **made-up
  conversations** (not the test workspace's existing messages); the user sees them before they are committed.
- Accuracy rule: every moved claim is checked against the code; a claim the code can't settle stays only with a record
  of being tried (README says so, a LIVE row in `PROGRESS.md`, a closed GitHub issue); otherwise reworded or "not tried
  yet". No new checks on the Windows VM or the Mac. Bugs found in ttyak → GitHub issues (public wording). Everything
  wrong in the in-app help (F6, `?`) → **one** GitHub issue.
- Live runs: only `--profile default` (account A) or `--profile b` (account B), never the default profile `test` (the
  user's work workspace) except read-only and unfocused; fake `XDG_CONFIG_HOME` (0700) in the scratchpad; windows on a
  special workspace (`hyprctl dispatch exec "[workspace special:<name> silent] kitty …"`); never capture the real
  screen, mic or camera; one test per Slack account at a time.
- Old names (klack, slacktui) only on lines marked `<!-- old-name-ok -->` (`tools/check-name.sh`).
- Ledger (not committed, `.superpowers/` is git-excluded): `.superpowers/sdd/2026-10-26-readme-wiki/` — `rules.md`
  (base brief: copy the constraints above), `accuracy.md` (per page: what was checked how, what changed, what stays
  unverified), `help-discrepancies.md` (the in-app help findings), `issues.md` (bugs to file).

## Wiki conventions (all tasks)

- One file per page: `docs/wiki/<Page-Name>.md`; the page title is the file name with `-` as spaces. Special files:
  `Home.md`, `_Sidebar.md`, `_Footer.md`.
- Links between pages: `[text](Page-Name)` or `[text](Page-Name#anchor)`. From the README and the site: absolute
  `https://github.com/dmytrobsp/ttyak/wiki/Page-Name#anchor`. Anchors follow GitHub's heading slugs.
- Images: `docs/images/<name>.<ext>` in the main repo; the README links them relatively, the wiki by
  `https://raw.githubusercontent.com/dmytrobsp/ttyak/master/docs/images/<name>.<ext>`.
- Each detailed page ends with `## If it doesn't work`, linking to its `Troubleshooting#…` entries (filled in Task 7).

## Page map (source lines of today's README)

| Page | README lines | Task |
|---|---|---|
| `Install` (one-liners, package managers, by hand + the download table, update, uninstall) | 14–70, 154–175 | 2 |
| `Requirements` | 71–132 | 2 |
| `Verifying-downloads` | 133–153 | 2 |
| `Building-from-source` | 176–206 | 2 |
| `Signing-in` (the guide, three ways, one workspace, from a browser, for scripts) | 247–302 | 3 |
| `Profiles-and-sessions` (session files, `auth status`/logout, `--profile`, several workspaces) | 303–340 | 3 |
| `Notifications-and-attention` (tab title, notifications, bell, sound, DND, through the terminal, clicks, macOS, Windows, commands) | 341–496 | 4 |
| `The-screen` (incl. statuses) | 207–246 | 5 |
| `Keys`, `Links-into-your-Slack`, `Rebinding-keys` | 497–602, 603–612, 613–648 | 5 |
| `Images-and-video` (pictures, the viewer, video playback — gathered from the screen/keys/requirements text) | scattered | 5 |
| `Huddles` | 649–729 | 6 |
| `Settings` (env variables table), `Updates`, `Themes`, `Translations` | 730–741, 742–763, 764–777, 778–783 | 6 |
| `Scripting` (`ctl`, status for bars) | 784–809 | 6 |
| `Project-layout`, `Contributing` (→ `CONTRIBUTING.md`) | 810–819 | 6 |
| `Home`, `_Sidebar`, `_Footer`, `Troubleshooting` | new | 7 |

The builder may split or merge pages (spec), keeping the groups and the order: Getting started, Using ttyak,
Configuring, Scripting, Developing, Troubleshooting (last).

## Review Focus

1. **A link that lands nowhere** — README → wiki page/anchor, wiki → wiki, site → wiki, and the anchors shipped
   binaries and manifests already use (`#requirements` from 0.12.1's font hint, `#verify` from the site, `#licence`
   from winget/Chocolatey). Expect every one to resolve. Pinned by `check_links.py` (Task 1) and the README-anchor
   test (Task 10).
2. **A private string in a public page or image** — a test-account name, Slack id, email, workspace name or machine
   detail in `docs/wiki/` or `docs/images/` (SVG text is text). Expect the publish to refuse. Pinned by the publish
   script running `tools/site/check_private.py` on both folders (Task 1) and the screenshot review (Task 8).
3. **The wiki showing an unreleased feature** — after this project, a publish from a master that documents new work.
   Expect the script to publish the latest release tag's `docs/wiki/` by default and say so. Pinned in Task 1's tests.
4. **The site changing when the README shrinks** — `fill.py` reads the download table from the README; after the move
   the site's download "Runs on / Needs" cells must stay the same. Pinned in Task 2 (fill output unchanged).
5. **A release zip without the README breaking install or update** — the install scripts and the self-update read
   the zip; one without `README.md` must install and update the same. Pinned in Task 11's install tests.

---

### Task 1: Wiki tooling — link checker and publish script

**Files:**
- Create: `tools/wiki/check_links.py`, `tools/wiki/test_check_links.py`, `tools/wiki/publish.sh`,
  `tools/wiki/test_publish.py`, `docs/wiki/Home.md` (a placeholder: title + one line, replaced in Task 7)
- Modify: `.github/workflows/ci.yml:34` (add the two test files to the unittest list), `tools/README.md` (one entry)

**Interfaces:**
- Produces: `check_links.slug(heading: str) -> str` (GitHub's heading slug: lowercase; drop every char that is not a
  letter, digit, space, `-` or `_`; spaces → `-`); `check_links.anchors(md_text: str) -> set[str]` (slugs of `#`
  headings outside code fences, duplicates as `x`, `x-1`, `x-2`; plus explicit `<a id="…">`/`<a name="…">`);
  `check_links.check(wiki: Path, readme: Path | None, extra: list[Path]) -> list[str]` (one human line per broken
  link: `file:line: target`); CLI `python3 tools/wiki/check_links.py [--wiki DIR] [--readme FILE] [EXTRA…]`, defaults
  `docs/wiki`, `README.md`, `site/index.html`; exit 1 when any problem.
- Checked link forms: relative `(Page)` / `(Page#a)` inside wiki pages; absolute
  `https://github.com/dmytrobsp/ttyak/wiki/Page#a` anywhere; `https://github.com/dmytrobsp/ttyak#a` (README anchors)
  anywhere; `#a` inside a file (its own anchors). Everything else (other URLs, `mailto:`) is ignored.
- Produces: `tools/wiki/publish.sh [--ref REF] [--push] [--wiki-dir DIR]` — REF defaults to the latest `v*` tag
  (`git describe --tags --abbrev=0 --match 'v*'`), `--ref master` for the first publish; it exports `docs/wiki/` at
  REF (`git archive`), runs `check_links.py` on the export and `python3 tools/site/check_private.py` on the export and
  on `docs/images/` (refusing on any finding), clones (or reuses `--wiki-dir`, default a fresh dir under
  `${TMPDIR:-/tmp}`) `https://github.com/dmytrobsp/ttyak.wiki.git`, mirrors the export into it (pages missing from the
  export are deleted from the wiki), commits `wiki: docs/wiki at <REF> (<short sha>)`, prints `git show --stat` and
  the diff, and pushes only with `--push`. It prints which REF it used on its first line. For tests, the environment
  overrides the wiki remote (`TTYAK_WIKI_URL`) and the privacy scanner (`TTYAK_CHECK_PRIVATE`, a script path).

- [ ] **Step 1: Write the failing tests** in `tools/wiki/test_check_links.py`:
  `test_slug` (`slug("If it doesn't work") == "if-it-doesnt-work"`, `slug("Status for bars & scripts") ==
  "status-for-bars--scripts"`, `slug("F6 → Keys") == "f6--keys"`); `test_anchors_skip_code_and_number_duplicates`
  (a fenced `# not` is ignored; two `## Linux` give `linux`, `linux-1`); `test_relative_wiki_links` (a page linking
  `(Other#x)` reports nothing when `Other.md` has `## x`, one line naming the file and line when the page or the anchor
  is missing); `test_absolute_wiki_and_readme_links` (a README linking `…/wiki/Install#update` and a site file linking
  `…/ttyak#verify`); `test_ignores_other_urls`; `test_cli_exit_code` (subprocess, 0 clean, 1 broken).
  `tools/wiki/test_publish.py`: build a temp git repo with a tag `v1.0.0` holding `docs/wiki/Home.md` "old" and a
  later master commit "new", a bare repo standing in for the wiki (publish.sh takes `TTYAK_WIKI_URL` from the
  environment for tests), and a fake `check_private.py` path via `TTYAK_CHECK_PRIVATE` (empty forbidden list):
  `test_default_ref_is_latest_tag` (first line names `v1.0.0`; the wiki clone gets "old"); `test_ref_master`;
  `test_no_push_without_flag` (the bare repo's log is unchanged); `test_push` ; `test_deleted_page_is_removed`;
  `test_refuses_on_broken_link` (exit ≠ 0, nothing committed).
- [ ] **Step 2: Run them, expect failures:** `python3 -m unittest tools/wiki/test_check_links.py
  tools/wiki/test_publish.py` → errors (modules/script missing).
- [ ] **Step 3: Implement `check_links.py` and `publish.sh`** (stdlib only; publish.sh `set -euo pipefail`, every
  `rm -rf` on `${VAR:?}` after checking the path lies under the temp dir it made).
- [ ] **Step 4: Run the tests, expect PASS;** then `python3 tools/wiki/check_links.py` on the repo → exit 0 (Home.md
  placeholder, README's existing `#` links resolve or are listed — fix the checker, not the README, if a real
  GitHub anchor is misread).
- [ ] **Step 5: Commit** (`tools/wiki/*`, `docs/wiki/Home.md`, `.github/workflows/ci.yml`, `tools/README.md`); push;
  watch CI.

### Task 2: Getting started pages — Install, Requirements, Verifying downloads, Building from source

**Files:**
- Create: `docs/wiki/Install.md`, `docs/wiki/Requirements.md`, `docs/wiki/Verifying-downloads.md`,
  `docs/wiki/Building-from-source.md`
- Modify: `tools/site/fill.py:139-152,363-364,380-401` (read the table from `docs/wiki/Install.md`),
  `tools/site/test_fill.py:83-95,219,236,445`, `.github/workflows/pages.yml:11` (`README.md` →
  `docs/wiki/Install.md`), `crates/ttyak/src/install.rs:443-448` (`include_str!("../../../docs/wiki/Install.md")`)

**Interfaces:**
- Produces: `docs/wiki/Install.md` holds, under a `## Install` heading, the download table in today's form (first
  column `` `<platform>.zip` ``, then "Runs on", "Needs") — `fill.readme_needs(text)` keeps its name and parsing
  (it is renamed `install_needs` only if every caller and test changes with it); every update and uninstall command of
  `install::Method::ALL` appears in backticks in `Install.md`.

- [ ] **Step 1: Accuracy check** of README lines 14–206 against the code (installers `site/install.sh`,
  `site/install.ps1`; `crates/ttyak/src/install.rs`, `update.rs`, `check.rs`; `packaging/`; release workflow), per the
  rule in Global Constraints; record in `accuracy.md`. The zips stop carrying the README from the next release
  (Task 11), and the wiki describes the latest release: the zip sentence reads "`ttyak` (`ttyak.exe`) and the two
  licences (0.12.1 and older: the README too)", true before and after.
- [ ] **Step 2: Write the four pages** from the checked text; each ends with `## If it doesn't work` (links filled in
  Task 7, a placeholder link to `Troubleshooting` for now — Task 7 creates it; until then `check_links.py` reports it,
  which is expected only for `Troubleshooting`).
- [ ] **Step 3: Re-point `fill.py`, its tests, `pages.yml` and the `install.rs` test** to `docs/wiki/Install.md`.
- [ ] **Step 4: Verify:** `python3 -m unittest discover -s tools/site -p 'test_*.py'` → OK;
  `cargo test -p ttyak names_and_commands` → ok; `python3 tools/site/fill.py site` then `git diff --stat site/` →
  no change (the download table cells are the same); `node --test tools/site/` → pass.
- [ ] **Step 5: Commit** pages + tooling separately (two commits: `docs: wiki — Getting started pages`, then
  `site, install: read the download table from docs/wiki/Install.md`); push; watch CI and the Pages run.

### Task 3: Signing in pages

**Files:** Create `docs/wiki/Signing-in.md`, `docs/wiki/Profiles-and-sessions.md`

- [ ] **Step 1: Accuracy check** of README lines 247–340 against `crates/slack-core` (auth, browser cookie readers)
  and `crates/ttyak` (the sign-in guide, `auth` subcommands, profiles); the two-factor/SSO claim gets an explicit
  check (the short README repeats it). Record in `accuracy.md`; help-text discrepancies in `help-discrepancies.md`.
- [ ] **Step 2: Write the pages.** `Signing-in` opens with the same warning as the README (not an official app, no
  "open in app" links, three ways) before the detail.
- [ ] **Step 3: Verify:** `python3 tools/wiki/check_links.py` → only `Troubleshooting` links reported;
  `ttyak auth login --help` and `ttyak auth --help` (a `cargo build` binary, fake `XDG_CONFIG_HOME`) agree with every
  flag the pages name.
- [ ] **Step 4: Commit** `docs: wiki — Signing in pages`; push.

### Task 4: Notifications and attention page

**Files:** Create `docs/wiki/Notifications-and-attention.md`

- [ ] **Step 1: Accuracy check** of README lines 341–496 against `crates/ttyak/src/tui/notify.rs` and the settings
  (names and defaults of `title_mark`, `bell`, `follow_system_dnd`, `sound_volume`, …), the platform notes against
  PROGRESS AT17 records. Record as before.
- [ ] **Step 2: Write the page** (subheadings per topic so Troubleshooting can link to `#…` anchors).
- [ ] **Step 3: Verify:** check_links as in Task 3; every settings key named exists (`grep` in `crates/ttyak/src`).
- [ ] **Step 4: Commit** `docs: wiki — Notifications and attention`; push.

### Task 5: Using ttyak pages — The screen, Keys, Links, Rebinding, Images and video

**Files:** Create `docs/wiki/The-screen.md`, `docs/wiki/Keys.md`, `docs/wiki/Links-into-your-Slack.md`,
`docs/wiki/Rebinding-keys.md`, `docs/wiki/Images-and-video.md`

- [ ] **Step 1: Accuracy check** of README lines 207–246 and 497–648 against the keymap and help code
  (`crates/ttyak/src/tui/` key tables, `ctl keys-template` output under a fake config); every key in the tables
  matches the default keymap. Images-and-video gathers the picture, viewer and video text (requirements' graphics and
  ffmpeg lines, `␣o`, the viewer keys) — nothing new.
- [ ] **Step 2: Write the pages.**
- [ ] **Step 3: Verify:** check_links as before; `ttyak ctl keys-template` diffed by eye against `Keys.md` /
  `Rebinding-keys.md` (each mismatch fixed in the page or logged in `help-discrepancies.md`).
- [ ] **Step 4: Commit** `docs: wiki — Using ttyak pages`; push.

### Task 6: Huddles, Configuring, Scripting, Developing pages

**Files:** Create `docs/wiki/Huddles.md`, `docs/wiki/Settings.md`, `docs/wiki/Updates.md`, `docs/wiki/Themes.md`,
`docs/wiki/Translations.md`, `docs/wiki/Scripting.md`, `docs/wiki/Project-layout.md`, `docs/wiki/Contributing.md`
(a short page: what `CONTRIBUTING.md` covers + its link; edits to the wiki go to `docs/wiki/` in the main repo)

- [ ] **Step 1: Accuracy check** of README lines 649–819 (`crates/huddle`, env variables — every `TTYAK_*` in the
  table exists in the code and every user-facing `TTYAK_*` in the code is in the table —, `update.rs`, themes,
  `ctl locale-template`, `ctl --help`, `ctl status --json` field names, the crate list). Record as before.
- [ ] **Step 2: Write the pages.**
- [ ] **Step 3: Verify:** check_links as before; `ttyak ctl --help` lists every subcommand `Scripting.md` names.
- [ ] **Step 4: Commit** `docs: wiki — Huddles, Configuring, Scripting, Developing`; push.

### Task 7: The catalogue — Home, sidebar, footer, Troubleshooting

**Files:** Create `docs/wiki/Troubleshooting.md`, `docs/wiki/_Sidebar.md`, `docs/wiki/_Footer.md`; modify
`docs/wiki/Home.md` and every page's `## If it doesn't work` section.

- [ ] **Step 1: Troubleshooting** — one `##` per symptom (spec's list), each: what you see, why, what to do, link to
  the detailed page. Sources only: the moved pages, the code's own error/hint strings, issues labelled `known-issue`
  (`gh issue list -R dmytrobsp/ttyak -l known-issue`).
- [ ] **Step 2: Home + `_Sidebar`** — the groups in order, Troubleshooting last; Home has one line per page.
  `_Footer`: "These pages describe the latest release and are generated from `docs/wiki/` in
  github.com/dmytrobsp/ttyak — edit them there."
- [ ] **Step 3: Fill every `## If it doesn't work`** with `Troubleshooting#…` links.
- [ ] **Step 4: Verify:** `python3 tools/wiki/check_links.py` → exit 0; `python3 tools/site/check_private.py
  docs/wiki` → clean; every README section of lines 71–819 is accounted for in some page (a checklist in
  `accuracy.md`, section by section).
- [ ] **Step 5: Commit** `docs: wiki — the catalogue and Troubleshooting`; push.

### Task 8: Screenshots

**Files:** Create `docs/images/*.svg` or `*.png` (2–4: the main screen, the sign-in guide, a huddle; optionally the
image viewer), a short `docs/images/README.md` on how they were made (no private details); maybe a small helper under
`tools/site/` (reuse `captures.py` / `ansi2html.py`).

- [ ] **Step 1: Set up a made-up conversation** in a fresh channel of the test workspace, posted from A
  (`--profile default`) then B (`--profile b`), never both at once, never `test`. Made-up people's names come from the
  capture-time rename (`~/.config/ttyak-site/rename.json`, outside the repo) — test accounts' names and the workspace
  name never appear unblurred.
- [ ] **Step 2: Capture** in a hidden kitty window (special workspace), under a fake `XDG_CONFIG_HOME` holding copies
  of only A/B's sessions; build the images; **blur the workspace name** (SVG: a blur filter over those cells; PNG:
  blurred region) — check it is unreadable at 2× zoom.
- [ ] **Step 3: Verify:** `python3 tools/site/check_private.py docs/images` → clean; look at each image at github
  width in light and dark backgrounds.
- [ ] **Step 4: Show the user the images** (paths in full) and wait for their okay; only then commit
  `docs: screenshots for the README`; push. Close the kitty windows.

### Task 9: First wiki publish (with the user)

- [ ] **Step 1:** Give the user the drafts to read (the `docs/wiki/` file list + a rendered preview if they want one)
  and collect changes; apply them as `docs: wiki — the user's review` commits.
- [ ] **Step 2:** Ask the user to create the first wiki page in the web UI (https://github.com/dmytrobsp/ttyak/wiki →
  "Create the first page", any text) so `ttyak.wiki.git` exists; confirm with `git ls-remote
  https://github.com/dmytrobsp/ttyak.wiki.git`.
- [ ] **Step 3:** `tools/wiki/publish.sh --ref master` → show the user the printed diff summary; with their word,
  `tools/wiki/publish.sh --ref master --push`.
- [ ] **Step 4: Verify:** open `https://github.com/dmytrobsp/ttyak/wiki` and three pages (WebFetch or `gh api`): the
  sidebar, an image, a cross-page anchor link work.

### Task 10: The short README

**Files:** Modify `README.md` (rewrite), `CONTRIBUTING.md:5,22` (wiki links); Create
`tools/wiki/test_readme.py` (and add it to `ci.yml:34`)

- [ ] **Step 1: Write the failing test** `tools/wiki/test_readme.py`: `test_short` (fewer than 200 lines);
  `test_section_order` (the `##`/`###` headings, in order, are exactly: `## What ttyak is`, `## Signing in`,
  `## Install`, `### Requirements`, `### Verify`, `### Update`, `### Uninstall`, `## Build and contribute`,
  `## Project layout`, `## Licence`); `test_shipped_anchors` (`anchors(README)` ⊇ {`requirements`, `verify`,
  `licence`}); `test_every_section_links_the_wiki` (each `##` section except Licence holds a
  `https://github.com/dmytrobsp/ttyak/wiki/` link); `test_links_resolve` (`check_links.check` on the real repo → []).
  Run → FAIL on today's README.
- [ ] **Step 2: Write the README** per spec Goals 1 (badges true and live — check each URL; screenshots from Task 8;
  the "how it differs" numbers linked to ttyak.sh, not copied; the Requirements short list "what and why").
- [ ] **Step 3: Verify:** the test → PASS; `cargo test -p ttyak names_and_commands` still ok (it reads
  `Install.md`); `tools/check-name.sh` → clean; view the rendered README (`gh api -X POST /markdown` or GitHub after
  push).
- [ ] **Step 4: Commit** `docs: README — the short front door; the detail lives in the wiki`; push; watch CI (the
  test file and ci.yml change run it).

### Task 11: Stop shipping the README; re-point the code strings

**Files:** Modify `.github/workflows/release.yml:210`, `packaging/templates/nfpm.yaml:33-37`,
`packaging/templates/PKGBUILD:24`, `packaging/test_render.py:251,301`, `tools/install/test_install_sh.py:79`,
`tools/install/test_install_ps1.py:155`, `packaging/templates/ttyak.nuspec:15` (`docsUrl` →
`https://github.com/dmytrobsp/ttyak/wiki`), `crates/ttyak/src/check.rs:68` (`FONTS_HINT` → "…the fonts listed on
the wiki's Requirements page (github.com/dmytrobsp/ttyak/wiki/Requirements)…"), `crates/ttyak/src/tui/notify.rs:2028`
(comment); any other user-facing "README" string found by `grep -rn README crates/`.

- [ ] **Step 1: Failing tests first:** `test_render.py` asserts `README.md` is **absent** from the PKGBUILD and
  nfpm output (`assertNotIn("README.md", pb)` / `y`); the install-script tests build zips **without** `README.md` and
  still install; a `check.rs` test asserts `FONTS_HINT` contains `wiki/Requirements` and not `README`. Run → FAIL.
- [ ] **Step 2: Change** the release staging line (binary + two licences), the templates, the nuspec, the strings.
- [ ] **Step 3: Verify:** `python3 -m unittest discover -s packaging -p 'test_*.py'`, the install tests,
  `cargo test -p ttyak` → all pass; `grep -rn 'README' crates/ packaging/templates .github/workflows` → no
  user-facing hit left (comments in `update.rs` fixtures that name a `README.md` entry stay: they test that extra files
  are ignored).
- [ ] **Step 4: Commit** `release: the zips and packages no longer carry the README; strings point at the wiki`;
  push; watch CI.

### Task 12: The site in line

**Files:** Modify `site/index.html:290,297`, `tools/site/fill.py:113-115` (`render_signing_note`'s `#verify` →
`https://github.com/dmytrobsp/ttyak/wiki/Verifying-downloads`), `tools/site/test_fill.py`, `tools/site/test_copy.py`
only if an approved line is touched (it must not be).

- [ ] **Step 1: Failing test** in `test_fill.py`: the rendered install block links
  `…/wiki/Verifying-downloads` and no `ttyak#verify`. Run → FAIL.
- [ ] **Step 2: Change** the requirements block's "The full list" link → `…/wiki/Requirements`, the footer's "the
  manual (the README)" → "the manual (the wiki)" with a link to `…/wiki`; compare the site's requirements and install
  notes with the checked wiki pages, fix facts that disagree (wording otherwise untouched; any change to wording beyond
  facts and links goes to the user first).
- [ ] **Step 3: Verify:** `python3 -m unittest discover -s tools/site -p 'test_*.py'`, `node --test tools/site/` →
  pass; `python3 tools/wiki/check_links.py` → 0; `python3 tools/site/check_private.py site` → clean.
- [ ] **Step 4: Commit** `site: links and facts point at the wiki`; push; watch the Pages run; fetch
  https://ttyak.sh and check the two links.

### Task 13: Wrap-up — release checklist, issues, recovery docs

**Files:** Modify `docs/RELEASING.md` (a step after the channel checks: `tools/wiki/publish.sh` at the new tag with the
user's word; a line in the dry run: "the zips hold `ttyak` and the two licences, no README"; a reminder to update the
wiki's zip sentence in `Install.md` when the first README-less release ships), `CONTRIBUTING.md` (the wiki pages are
edited in `docs/wiki/` and published at a release), `PROGRESS.md` (a row and the NEXT paragraph, honest LIVE/UNIT/NOT).

- [ ] **Step 1:** File **one** issue for the in-app help discrepancies (`help-discrepancies.md`, public wording,
  labels `minor`, the right `area:*`, milestone `later`), and one issue per ttyak bug in `issues.md`.
- [ ] **Step 2:** Edit RELEASING.md, CONTRIBUTING.md, PROGRESS.md.
- [ ] **Step 3: Verify:** `python3 tools/wiki/check_links.py` → 0; `tools/check-name.sh` → clean; `git status`
  clean after the commit.
- [ ] **Step 4: Commit** `docs: RELEASING/CONTRIBUTING — the wiki; PROGRESS — the README/wiki split`; push.
