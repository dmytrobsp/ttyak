# README → short README + GitHub wiki catalogue — design (2026-10-26)

## Intent

The user: "readme is very long and contains too much unnecessary detailed information. i'm not even sure its
up-to-date. i want to split it so readme stays relatively short. instead we can leverage github wiki to build a
catalogue of detailed pages on various topics".

Today `README.md` is 825 lines and is the whole manual: install, requirements, verify, update, uninstall, build, the
screen, signing in (~250 lines), keys, links, rebinding, huddles, settings, updates, themes, translations, scripting,
status for bars, layout, licence. The site (ttyak.sh, `site/index.html`) repeats parts of it and calls the README "the
manual".

After this work:

- **The README is a front door.** Someone landing on github.com/dmytrobsp/ttyak learns in a minute what ttyak is,
  how it differs from Slack's official app, that signing in is not as smooth as in the official app, and how to
  install, update, uninstall, build and contribute. Every section is intentionally short and ends with a link to
  the wiki page that covers the subject in detail.
- **The wiki is the manual**, a catalogue of detailed pages. Its typical reader (the user's words) is "someone who
  got stuck on some sort of problem".
- **Everything moved is checked against the code** on the way ("i'm not even sure its up-to-date").
- **The site is brought in line** (the user: "include the site"): its links and its "manual" wording point at the
  wiki, and the facts it repeats agree with the checked pages.

## Goals

1. A short README with these sections, in this order (the user's list, plus their sign-in warning "somewhere at the
   top"):
   1. **Title, one-line description, badges.** Badges "sprinkled on top" (the user). Builder's choice of which; each
      one must be true and live (e.g. latest release, CI status, licence, the package channels that really list
      ttyak; winget only once it is listed). Link to ttyak.sh and to the wiki's Home.
   2. **Screenshots.** A few (2–4) real images of ttyak. See "Screenshots".
   3. **What ttyak is and how it differs from the original** (Slack's official desktop app): a terminal client in
      Rust; what it does in one or two sentences; far lighter than the official app (the measured numbers live on
      the site: link, don't copy); not an official Slack app, not affiliated with Slack.
   4. **Signing in — read this first.** A warning, not a how-to. The user: "because we're not an official app and not
      a GUI app, we often cannot handle deeplinks and cant use regular slack logins. so the user has to be warned
      beforehand that his login process will not be as smooth as on the original." Say why (a browser session — the
      `xoxc` token plus the `d` cookie — instead of Slack's own sign-in; no "open in app" links), name the three
      ways in one line each (a code from your email, your browser, a pasted cookie), say that two-factor/SSO
      workspaces need the browser way (verify against the code first), link to the wiki's Signing in page.
   5. **Install / update / uninstall.** The one-line installers, the package-manager table (kept compact), one line
      each for update and uninstall → the wiki's Install page. It keeps two one-line subheadings, **Requirements**
      and **Verify**, each pointing at the wiki (my call): the released 0.12.1 binary says "install the fonts listed
      under Requirements in the README", and the site links to `#requirements` and `#verify` (`fill.py`
      `render_signing_note`), so those anchors must keep landing somewhere useful.
   6. **Build it yourself and contributing** → the wiki's Building page and `CONTRIBUTING.md`.
   7. **Project layout.** The crates and `vendor/rtc-turn`, a few lines → the wiki's Project layout page.
   8. **Licence.** As today, short.
2. A wiki catalogue (`Home`, plus a sidebar listing the same) grouped as below, with **Troubleshooting last** (the
   user: "a catalogue should still list troubleshooting somewhere at the end as is customary"):
   - **Getting started:** Install (requirements, every platform, verifying downloads, update, uninstall), Signing in
     (all three ways, one workspace, from a browser, for scripts, profiles, signing out).
   - **Using ttyak:** The screen, Keys (and rebinding), Links into your Slack, Images and video, Notifications and
     attention, Huddles.
   - **Configuring:** Settings, Themes, Translations, Updates.
   - **Scripting:** `ttyak ctl`, status for bars and scripts.
   - **Developing:** Building from source, Project layout, Contributing (a link to `CONTRIBUTING.md`).
   - **Troubleshooting:** one page, one heading per symptom (sign-in fails or a browser isn't found, pictures don't
     show, emoji as boxes, no notifications or no sound, huddle audio, Windows SmartScreen / Chrome, …): what you
     see, why, what to do, and a link to the detailed page. Each detailed page ends with a short "If it doesn't
     work" section linking to the matching Troubleshooting entries.
   The page list is the shape, not a contract: the builder may split a page that is too long (Signing in is ~250
   lines today) or merge two thin ones, as long as the groups and the order hold.
3. **Every moved section checked against the code** (see "The accuracy check").
4. **The site in line** (see "The site").
5. **Nothing that reads the README breaks** (see "What depends on the README today").

## Non-goals

- The in-app help (F6, `?`) is not rewritten (my call, told to the user). Anything wrong found there becomes a
  GitHub issue. The exception: code strings that point the user *at the README* (below) are changed, since they
  would be wrong after the split.
- No new features. The Troubleshooting page and the "If it doesn't work" sections are assembled from what the README,
  the code and the GitHub issues (label `known-issue`) already know; nothing is invented.
- No change to the site's design, its look or its approved jokes (`tools/site/test_copy.py` keeps the user's approved
  lines verbatim; any change to the page's own wording beyond links and facts goes to the user first).
- No translation of the wiki.

## Anti-goals (would be a failure even if it "works")

- A README that is long again: the detail creeps back in. Every section stays a few lines plus a link.
- A wiki page that states something untried as tried, or a fact that disagrees with the code.
- A public page or screenshot that shows the user's work workspace, test-account names, Slack ids, emails, or
  machine details.
- Broken links: README → wiki, wiki → wiki, site → README/wiki, the Chocolatey `docsUrl`, code strings.
- A broken release: the zips, the `.deb`/`.rpm`/AUR packages ship `README.md`; tests read it.
- Wiki edits made in GitHub's web editor silently overwritten without anyone knowing that's the rule.

## Constraints

- **Public and outward-facing.** Publishing wiki pages is public: the user sees the drafts before the first push to
  the wiki and okays each publish while this project runs. README/site pushes to master follow the usual rule
  (routine pushes need no asking).
- **The wiki repository does not exist yet.** The wiki is enabled on dmytrobsp/ttyak but empty;
  `dmytrobsp/ttyak.wiki.git` appears only after a first page is created in the web UI. The user may have to create
  that first page (any content; it is replaced on the first publish). Ask them when the publish step comes.
- **Wiki source lives in the main repo** (my call, told to the user and accepted): `docs/wiki/*.md`, one file per
  page in GitHub-wiki naming (`Home.md`, `_Sidebar.md`, `_Footer.md` optional, `Signing-in.md`, …; links in wiki
  form so they work on the wiki, not in the repo view). A small script (`tools/wiki/publish.sh` or similar) clones
  the wiki repo into the scratchpad, mirrors `docs/wiki/` into it, shows the diff, and pushes only when asked. The
  wiki stays editable by collaborators only (GitHub's default); `Home.md` (or the footer) says the pages are
  generated from `docs/wiki/` in the main repo and edits belong there.
- **The public wiki describes the latest release** (the user, 2026-10-26: "wiki should describe the latest
  release"). `docs/wiki/` on master may run ahead (new features documented with their code); the wiki is republished
  as a step of each release (added to `docs/RELEASING.md`; `CONTRIBUTING.md` says pages are edited in `docs/wiki/`).
  Between releases only fixes to released behaviour are published, and never a page that describes something
  unreleased: the publish script defaults to `docs/wiki/` as of the latest release tag, or shows the diff against it
  so the publisher sees what is unreleased (the builder picks the mechanism). The first publish is from master: there
  are no code changes since v0.12.1 (checked: `git diff v0.12.1..HEAD -- crates tools packaging site .github` is
  empty), and this project's own small code-string changes are about links, not features. No workflow and no
  wiki-write token (my call).
- **No release in this project** (my call; tags need the user's word anyway). The code-string and packaging changes
  (`FONTS_HINT`, the nuspec `docsUrl`) reach users with the next release.
- `docs/**` and `**.md` are paths-ignored by CI: docs-only commits skip CI. Commits that touch code or tests
  (`crates/`, `tools/site/`, `packaging/`, workflows) run CI: watch it (one capped waiter; `gh run watch` is fine).
- Small verified steps, one commit each; commit with explicit paths; no `cargo fmt`; never build `target/release`.
- Live checks (running ttyak to check a claim, making screenshots): only the test workspace (`--profile default` =
  account A, `--profile b` = account B), never the machine's default profile `test` (the user's work workspace) for
  anything but read-only use; a fake `XDG_CONFIG_HOME` (0700) in the scratchpad; windows hidden on a special
  workspace (`hyprctl dispatch exec "[workspace special:<name> silent] kitty …"`); never capture the real screen,
  mic or camera; one test per Slack account at a time.
- Old project names (klack, slacktui) appear only on lines marked `<!-- old-name-ok -->` (`tools/check-name.sh`).

## What depends on the README today (each must be handled)

- `crates/ttyak/src/install.rs` test `names_and_commands`: `include_str!` of `README.md`, asserts every update and
  uninstall command appears in backticks. → Point it at the page that now holds those commands (the README keeps one
  line each, so either works; the wiki Install page is the complete list).
- `tools/site/fill.py` `readme_needs`: reads the "Runs on / Needs" columns of the download table under `## Install`
  in `README.md` to fill the site's download table; `tools/site/test_fill.py` tests it against the real README;
  `.github/workflows/pages.yml` redeploys the site when `README.md` changes. → Read the table from wherever it lives
  now (likely `docs/wiki/Install.md`), update the tests and the workflow's `paths`.
- `crates/ttyak/src/check.rs` `FONTS_HINT`: "install the fonts listed under Requirements in the README" → the wiki
  page (URL or name). Grep `crates/` for other user-facing "README" strings; comments that say "as the README says"
  are updated to the new place.
- `packaging/templates/ttyak.nuspec` `docsUrl` → the wiki Home.
- `.github/workflows/release.yml` copies `README.md` into each zip; `packaging/templates/nfpm.yaml` and `PKGBUILD`
  install it to `/usr/share/doc/ttyak/`; `tools/install/test_install_*.py` expect it in the zip. → Unchanged: the
  short README ships, and its links lead to the wiki (my call: offline readers get the front door, not the manual).
- `CONTRIBUTING.md` points at the README's Requirements and "Signing in" → the wiki pages.
- The site: the "Requirements" block ends with a link to `github.com/dmytrobsp/ttyak#requirements`; the footer says
  "the manual (the README)".

## The accuracy check

For every README section that moves (and the facts the short README keeps), the builder checks each claim before it
goes into a page:

- Against the code (the key tables, settings names and defaults, CLI flags, file paths, the sign-in flow, platform
  notes), with `ttyak --help` / `ttyak <cmd> --help` / `ttyak ctl check` output where it helps, run under a fake
  config folder.
- A claim the code contradicts is fixed in the page. A claim that can't be checked from the code (a live session,
  another machine) stays if it has a record of being tried: the README saying so, a LIVE row in `PROGRESS.md`, or a
  closed GitHub issue (my call). Without any record it is reworded to what is known or marked "not tried yet", as
  the README already does elsewhere. No new live checks on the Windows VM or the Mac in this project (my call); live
  runs on this machine (the test workspace, a fake config folder) are fine where cheap.
- A finding that shows a bug in ttyak (not in the docs) becomes a GitHub issue (public wording), not a fix here.
- The check leaves a short record per page (in the plan's ledger, not in the wiki): what was checked how, what
  changed, what stays unverified.

## Screenshots

2–4 real images for the README (and reused by the wiki where useful): the main screen (sidebar, a conversation, a
thread), the sign-in guide, a huddle; the builder may swap one for the image viewer. Made in the test workspace only,
with the same care as the site's captures (`tools/site/CAPTURES.md`: raw kitty captures, the private rename/fixes
files outside the repo, nothing personal on screen). The builder chooses the route: the existing capture pipeline
(`tools/site/captures.py` / `ansi2html.py` already turn a kitty capture into an SVG of the cell grid; a standalone
SVG renders in a GitHub README as an `<img>`) or PNGs from a hidden kitty window — whichever gives a faithful,
legible image on github.com in light and dark mode. Images live in the main repo (e.g. `docs/images/`), and the user
sees them before they are committed.

## The site

- Its link to `#requirements` and the footer's "the manual (the README)" point at the wiki.
- The facts it repeats (the requirements block, install notes, the downloads note) are compared with the checked
  wiki pages and fixed where they disagree; the page's approved wording and look stay.
- Its tests (`tools/site/test_*.py`, `*.test.mjs`) stay green.

## Left to the builder

- Exact page names, which long pages to split, page wording, the badge set, the README's exact length (aim: roughly
  100–150 lines; it must read as short).
- The publish script's shape and name; link form inside wiki pages.
- The order of the work, as long as each step is small, verified and committed on its own, and nothing half-moved is
  pushed (e.g. the README is cut only once the wiki pages holding its content exist in `docs/wiki/`).

## Calls I made (for the user to check)

- Wiki source in `docs/wiki/`, published by a script; web edits are overwritten (accepted in the conversation).
- The in-app help is out of scope except strings that point at the README.
- The zips and Linux packages keep shipping the (now short) README.
- Screenshots from the test workspace only, shown to the user before committing.
- What counts as "tried": the README, a LIVE row in PROGRESS.md, or a closed issue; no new VM/Mac checks.
- The short README keeps one-line Requirements and Verify subheadings (anchors the 0.12.1 binary and the site use).
- No release in this project; the wiki is republished at each release by a script, no workflow or token.
