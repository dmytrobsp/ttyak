#!/bin/sh
# Usage: sh tools/launch.sh PROFILE [DM_NAME]      (or TTYAK_TEST_PROFILE / TTYAK_TEST_DM in the environment)
# Launch ttyak (debug build) on PROFILE — a test profile named in AGENTS.local.md, never one you were not given —
# in a fresh kitty window (class ttyak-test, socket unix:@ttyak-test); with DM_NAME, open that DM (Ctrl+K, the name,
# Enter).
# The window is VISIBLE on the current workspace: use it only while the maintainer watches
# (docs/agents/live-testing.md has the hidden-window recipe). Extra env (e.g. TTYAK_LOG=ttyak=debug) is passed
# through. TTYAK_FAKE_HOME is required, an absolute path: a fake home made by
# `python3 tools/accounts/accounts.py install ACCOUNT ID --home DIR` under a claim; ttyak runs with HOME and XDG_*
# pointing into it, never in the real config folder.
# Run `cargo build` first. Needs Hyprland (hyprctl), jq and kitty.
set -u

profile=${1:-${TTYAK_TEST_PROFILE:-}}
dm=${2:-${TTYAK_TEST_DM:-}}
case $profile in
    "" | -*)
        echo "usage: sh tools/launch.sh PROFILE [DM_NAME] — PROFILE is a test profile from AGENTS.local.md" >&2
        exit 2 ;;
esac

fake=${TTYAK_FAKE_HOME:-}
hint="claim the account, then install it first:
  python3 tools/accounts/accounts.py claim ACCOUNT ...
  python3 tools/accounts/accounts.py install ACCOUNT ID --home DIR"
if [ -z "$fake" ]; then
    echo "TTYAK_FAKE_HOME is not set: $hint" >&2
    exit 2
fi
case $fake in
    /*) ;;
    *) echo "TTYAK_FAKE_HOME must be an absolute path: $hint" >&2
       exit 2 ;;
esac
if [ ! -d "$fake" ] || [ ! -f "$fake/.config/ttyak/$profile/session.json" ]; then
    echo "TTYAK_FAKE_HOME has no .config/ttyak/$profile/session.json: $hint" >&2
    exit 2
fi

hyprctl dispatch closewindow class:ttyak-test >/dev/null
timeout 10 sh -c 'while hyprctl clients -j | jq -e ".[] | select(.class==\"ttyak-test\")" >/dev/null; do
    sleep 0.3
done'
cd "$(dirname "$0")/.." || exit 2
# The profile and the fake home reach the window's shell through the environment, never through the quoted command;
# HOME and XDG_* are set on the ttyak command only.
(TTYAK_LAUNCH_PROFILE=$profile TTYAK_FAKE_HOME=$fake kitty --class ttyak-test \
    -o allow_remote_control=socket-only --listen-on unix:@ttyak-test \
    sh -c 'HOME="$TTYAK_FAKE_HOME" \
XDG_CONFIG_HOME="$TTYAK_FAKE_HOME/.config" \
XDG_DATA_HOME="$TTYAK_FAKE_HOME/.local/share" \
XDG_CACHE_HOME="$TTYAK_FAKE_HOME/.cache" \
./target/debug/ttyak --profile "$TTYAK_LAUNCH_PROFILE"; exec sleep 600' >/dev/null 2>&1 &)
timeout 10 sh -c 'until kitty @ --to unix:@ttyak-test ls >/dev/null 2>&1; do sleep 0.3; done'
[ -n "$dm" ] || exit 0
timeout 5 sh -c 'sleep 3'
kitty @ --to unix:@ttyak-test send-key ctrl+k
kitty @ --to unix:@ttyak-test send-text "$dm"
kitty @ --to unix:@ttyak-test send-key enter
timeout 3 sh -c 'sleep 2'
