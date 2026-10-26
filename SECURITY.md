# Security

## Reporting a vulnerability

Please report it privately, never in a public issue, pull request or discussion:
[report a vulnerability](https://github.com/dmytrobsp/ttyak/security/advisories/new) (the repository's Security tab →
Advisories → Report a vulnerability). Only the maintainer sees the report; it becomes public as an advisory once a
fixed release is out.

Say what you found, how to reproduce it, which version (`ttyak --version`) and system, and what it gives an attacker.
Do not send tokens, cookies or session files, yours or anyone's.

## Supported versions

Fixes go into the next release; only the [latest release](https://github.com/dmytrobsp/ttyak/releases/latest) is
supported. ttyak says when a newer one is out, and F6 → Updates shows how to update this copy (see the wiki's
[Install](https://github.com/dmytrobsp/ttyak/wiki/Install#update)).

## What counts

Anything that could leak a Slack session (the `xoxc` token, the `d` cookie, `session.json`), let someone else read or
change your files through ttyak, run code through a message, a file or an update, or weaken the update's checks.
Slack's own bugs go to Slack.
