#!/bin/sh
# Usage: sh tools/launch.sh PROFILE [DM_NAME]      (or TTYAK_TEST_PROFILE / TTYAK_TEST_DM in the environment)
# Launch ttyak (debug build) on PROFILE — a test profile named in AGENTS.local.md, never one you were not given — in a
# fresh kitty window (class ttyak-test, socket unix:@ttyak-test); with DM_NAME, open that DM (Ctrl+K, the name, Enter).
# The window is VISIBLE on the current workspace: use it only while the maintainer watches
# (docs/agents/live-testing.md has the hidden-window recipe). Extra env (e.g. TTYAK_LOG=ttyak=debug) is passed through.
# Run `cargo build` first. Needs Hyprland (hyprctl), jq and kitty.
set -u

profile=${1:-${TTYAK_TEST_PROFILE:-}}
dm=${2:-${TTYAK_TEST_DM:-}}
case $profile in
    "" | -*)
        echo "usage: sh tools/launch.sh PROFILE [DM_NAME] — PROFILE is a test profile from AGENTS.local.md" >&2
        exit 2 ;;
esac

hyprctl dispatch closewindow class:ttyak-test >/dev/null
timeout 10 sh -c 'while hyprctl clients -j | jq -e ".[] | select(.class==\"ttyak-test\")" >/dev/null; do sleep 0.3; done'
cd "$(dirname "$0")/.." || exit 2
# The profile reaches the window's shell through the environment, never through the quoted command.
(TTYAK_LAUNCH_PROFILE=$profile kitty --class ttyak-test -o allow_remote_control=socket-only --listen-on unix:@ttyak-test \
    sh -c './target/debug/ttyak --profile "$TTYAK_LAUNCH_PROFILE"; exec sleep 600' >/dev/null 2>&1 &)
timeout 10 sh -c 'until kitty @ --to unix:@ttyak-test ls >/dev/null 2>&1; do sleep 0.3; done'
[ -n "$dm" ] || exit 0
timeout 5 sh -c 'sleep 3'
kitty @ --to unix:@ttyak-test send-key ctrl+k
kitty @ --to unix:@ttyak-test send-text "$dm"
kitty @ --to unix:@ttyak-test send-key enter
timeout 3 sh -c 'sleep 2'
