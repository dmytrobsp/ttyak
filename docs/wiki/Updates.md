ttyak looks for a new version by itself and says so once. How to update each kind of install is on
[Install](Install#update); this page is about the check and F6 → Updates.

## The check

Once a day ttyak makes a plain GET of `https://ttyak.sh/latest.json`, which sends nothing about you but ttyak's name and
version as the User-Agent (no cookies). Pre-releases are never offered. A check that gets no answer counts as done:
the next one comes a whole interval later.

A new version is told once, ever:

- a desktop notification, when they are on (it never sounds); on Linux desktops a click on it opens the update screen
  (on macOS and Windows a click is meant to do the same; not tried yet)
- `⬆ <version>` in the status line (a click opens the update screen too). It comes back after a restart until you
  open the update screen or that version is installed.

What was told, the newer version found and the last one whose update screen you opened are kept in `updates.json` in
ttyak's data folder (`~/.local/share/ttyak/` on Linux). A check that finds nothing newer forgets the version found;
with the check off nothing does, so a version found before is offered from the start of every run, even one no longer
out, until Check now says otherwise.

## F6 → Updates

- **Version**: this ttyak's version and how it was installed: the install script, Homebrew, Scoop, winget, Chocolatey,
  the Arch package, a .deb or .rpm package, or manual download for a copy downloaded by hand.
- **Check for updates**: on (the default) or off (`[updates] check` in `settings.toml`).
- **Check at most every**: an hour to a week, a day by default (`check_every_secs`).
- **Check timeout**: how long the check waits for an answer (`check_timeout_secs`, 10 seconds).
- **Download timeout**: how long the update screen's download waits for data (`download_timeout_secs`, 30 seconds).
- **Check now** looks at once, due or not and with the check off too, and says what it found on its row — never with a
  desktop notification. A version it finds counts as told, so no notification or `⬆` comes for it later either, but
  the Update row stays while it is known, in a later run from the start.
- **To update it**: what to run to update this install (Enter copies it), or the page to download from.
- **Update**, only while a newer version is known: opens the update screen.

Hand-edited seconds below the least F6 offers count as it: an hour between checks, 5 s for the check's answer (which
waits 10 minutes at most), 10 s for the download (which waits a day at most).

## The update screen

It shows the new version, its date and the version you have, with a link to the release notes. Then:

- a copy the install script put in a folder ttyak can write: **Update now** downloads the new version, checks it
  against the release's `SHA256SUMS` (on macOS also its signature: see [Verifying downloads](Verifying-downloads)) and
  puts it in place. ttyak never restarts by itself: restart it when you are ready. Cancel (or Esc) during the download
  stops it, and nothing is changed.
- a copy the install script put in a folder ttyak cannot write: the install one-liner for that folder, to run as
  someone who can write there
- Homebrew, Scoop, winget, Chocolatey: the command to run (Copy the command)
- a .deb or .rpm package, or a copy downloaded by hand: the release page to download from (Copy the link); the Arch
  package: the recipes zip to build again with `makepkg -si`

## If it doesn't work

- [Troubleshooting](Troubleshooting)
