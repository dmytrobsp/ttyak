# Releasing ttyak

The release itself is automated: pushing a tag `vX.Y.Z` runs `.github/workflows/release.yml`, which builds every
platform, signs and notarizes the macOS builds, creates the GitHub Release and publishes to the Homebrew tap, the Scoop
bucket, winget, Chocolatey (and the AUR, once it is set up), then the site's Pages run serves the new
`https://ttyak.sh/latest.json`. This page is everything around it, in order. Accounts and secrets:
[distribution/accounts.md](distribution/accounts.md).

A tag is pushed only with the maintainer's word.

## 1. Decide what goes in

- The release's milestone on GitHub (`X.Y.Z`) lists what it should contain:
  `gh issue list -R dmytrobsp/ttyak -m X.Y.Z --state all`.
- Every issue in it is closed, or moved to `later` with a note why. Only release blockers hold a tag up; polish waits
  for the next milestone.
- Fixes on master are reviewed and CI is green on the commit to be tagged.

## 2. Bump the version

- `Cargo.toml` (`[workspace.package] version`) and `crates/ttyak/Cargo.toml` (the `slack-core` dependency's version),
  then `cargo build` to update `Cargo.lock`. Check: `./target/debug/ttyak --version`.
- The attestation example in `docs/wiki/Verifying-downloads.md` names X.Y.Z (see step 7).
- The whole `cargo test`, then one commit `release: X.Y.Z — <one line>` with those four files, pushed to master.
- Wait for CI on it (one waiter: `gh run watch <id> --exit-status`).

## 3. Dry run

- `gh workflow run release.yml -R dmytrobsp/ttyak --ref master -f sign=false`: the same optimised builds for all five
  platforms plus the packaging and winget's dry run; it publishes nothing.
- Worth it whenever the release changes dependencies, platform code or the workflow itself: release builds differ
  from CI's (fat LTO, the Windows build is optimised), and a failure after the tag fails mid-publish.
- Look at the packaging step's output: the zips hold `ttyak` (`ttyak.exe`) and the two licences, no README (true from
  the release after 0.12.1; the first one that ships this is the check that `release.yml` really stopped adding it).
- Typical times (0.12.1): Windows 4.6 min, aarch64 Linux 4.9, x86_64 Linux 8.0, x86_64 macOS 8.2, aarch64 macOS 9.5.

## 4. Release notes

- Draft them before the tag: what users notice first, grouped (clicks, do-not-disturb, platforms, Also), a
  **Known issue** line for open `known-issue`s that matter, and **Tried for this release** — only what was checked
  live (README and notes never state something untried as tried).
- End with the install line: "Install or update as before: https://ttyak.sh — installed with the script, ttyak offers
  the update itself (F6 → Updates)." and the Full Changelog link.

## 5. Tag

- `git tag -a vX.Y.Z -m "ttyak X.Y.Z — <one line>" <bump commit>` and `git push origin vX.Y.Z`.
- Watch the release run (one waiter). macOS notarization: both builds should say `Accepted` in the Notarize step.
- When the GitHub Release exists, set the notes: `gh release edit vX.Y.Z -R dmytrobsp/ttyak --notes-file <file>`.
- If a publish job fails: fix, then re-run only the failed jobs (`gh run rerun <id> --failed`). Delete a partial
  GitHub Release before re-running the whole Publish.

## 6. Check every channel

- `https://ttyak.sh/latest.json` says X.Y.Z (after the Pages run).
- A fresh install from the live site into a throwaway home, never the maintainer's own copy:
  `env -i PATH=/usr/bin:/bin HOME=<tmp>/home sh install.sh --yes --dir <tmp>/bin --no-modify-path`, then
  `<tmp>/bin/ttyak --version`.
- Homebrew: `dmytrobsp/homebrew-ttyak` `Formula/ttyak.rb` points at the vX.Y.Z archives. Scoop:
  `dmytrobsp/scoop-ttyak` `bucket/ttyak.json` says X.Y.Z.
- winget: the release run opens a PR on `microsoft/winget-pkgs`. If the bot asks for the CLA, the maintainer (or the
  agent, with the maintainer's word) comments `@microsoft-github-policy-service agree`. Close an older ttyak PR that
  this one supersedes. Then it waits for a volunteer moderator.
- Chocolatey: each version is moderated by hand; a new version is refused while an older one is still in moderation —
  skip a version rather than queue it behind the next.

## 7. Publish the wiki

- The wiki (https://github.com/dmytrobsp/ttyak/wiki) is the user manual; its pages live in `docs/wiki/`. With the
  maintainer's word, publish them at the new tag: `tools/wiki/publish.sh` (it defaults to the latest `v*` tag; it
  prints the identity it commits as and the wiki remote, and pushes nothing). Then `tools/wiki/publish.sh --push` —
  only with the maintainer's word.
- The attestation example in `docs/wiki/Verifying-downloads.md` names the new version (the "for version …" line, the
  zip name and `refs/tags/v…`, and the tag in the sentence below it). The wiki is published from the tag, so that
  edit is part of the bump (step 2). Before publishing, run the command on the new zip, and once with another tag,
  and check that the quoted failure wording ("expected SourceRepositoryRef to be refs/tags/…") still matches what
  `gh` prints; if it doesn't, fix the page on master (it goes out with the next publish).
- The first release without the README in its zips and packages: the zip sentence in `docs/wiki/Install.md` must read
  right ("0.12.1 and older: the README too" can stay).
- Once winget lists ttyak, the lines that state winget's status change to say it is listed: the winget row of the
  package table in `README.md` ("not listed yet …"), `docs/wiki/Install.md` ("winget has not listed ttyak yet …",
  under the package table), `docs/wiki/Troubleshooting.md` ("winget finds nothing": "winget has not listed ttyak
  yet …"), and on the site `tools/site/fill.py` (the winget item of the package list, "not listed yet (its first version is waiting for review)", and the
  note "Chocolatey reviews each new version: it can list it a few days after the release"; check both still read right),
  then `python3 tools/site/fill.py site`.

## 8. Close out

- Close the milestone: `gh api -X PATCH repos/dmytrobsp/ttyak/milestones/<number> -f state=closed`
  (`gh api repos/dmytrobsp/ttyak/milestones` lists the numbers). Create the next one when its first issue is picked.
- PROGRESS.md: a row for the release (runs, timings, what was checked live and what not) and the NEXT UP paragraph.
- Outside waits (winget and Chocolatey moderation) go into NEXT UP until they are done.
