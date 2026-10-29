# ttyak's patch to crossterm 0.29.0

This folder is crossterm 0.29.0 exactly as published on crates.io, plus the changes below. Every changed
line is marked `// ttyak:`. `Cargo.toml` at the root of ttyak points at it with
`[patch.crates-io] crossterm = { path = "vendor/crossterm" }`. The crate is MIT (`LICENSE` here).

## Why

A terminal with SGR mouse reporting (`CSI < Cb ; Cx ; Cy M/m`) sends the mouse's side buttons as buttons 8
and 9 (`Cb` 128 and 129, plus 32 for a drag). crossterm 0.29's `parse_cb` answers those with a parse error,
so the event is lost and ttyak never sees "back" or "forward". browsers and X11/Wayland call button 8 back
and button 9 forward.

Also: kitty answers a clicked desktop notification with an OSC reply (`ESC ] 99 ; … ESC \`). crossterm has no OSC
parsing, so it read that as Alt+] followed by the typed characters `99;…` and a stray `\`. ttyak needs the reply
as one event (Task 9 of 0.12.0).

## What changed

- `MouseButton` (`src/event.rs`) gains `Back` and `Forward`, and `Button10` and `Button11` (named by their SGR
  button number, as the mouse has no common name for them).
- `parse_cb` (`src/event/sys/unix/parse.rs`) decodes buttons 8 and 9 as `Down`, buttons 10 and 11 (`Cb` 130 and 131)
  the same, and all four as `Drag` with the motion bit; the release (`m`) turns `Down` into `Up` as for the other
  buttons. Button 12 and up (`Cb` 192 and up) still fail. ttyak ignores 10 and 11 on purpose.
- `Event::Osc(String)` (`src/event.rs`): the payload between `ESC ]` and its end (BEL or `ESC \`), both left out. Only
  the Unix backend produces it; the Windows backend never does, so it compiles unchanged. `Event` is no longer
  `Copy` without the `bracketed-paste` feature (it carries a `String`; ttyak uses the default features, where it
  was not `Copy` already).
- `parse_event` (`parse.rs`): `ESC ] 9` starts an OSC, read to BEL or `ESC \` (`None` until then). `ESC ]` with no more
  input pending stays Alt+], as does `ESC ]` and any other byte.
- Only kitty's notification report is read: `ESC ] 99 ;` then printable ASCII without space (`0x21..=0x7E`), ended by
  BEL or `ESC \`, at most 256 bytes in all. `advance_bytes` (`parse.rs`, shared by the two readers `source/unix/mio.rs`
  and `tty.rs`, which used to have the loop each) gives an OSC up on the first byte that cannot belong to it (anything
  not `99;` right after `ESC ]`, a control byte, space, non-ASCII, an `ESC` not followed by `\`, the 257th byte) and
  also when the read ends with the OSC incomplete and nothing more to read (so `parse_osc` never sees more than 256
  bytes and has no length check of its own). Giving up means the `ESC ]` is Alt+] and every byte after it is fed
  again as ordinary input, so nothing is lost and nothing is swallowed: a stray `ESC ] 9 9 ; h` is Alt+], `9`, `9`,
  `;`, `h`, and a mouse report or arrow after a stray OSC stays a mouse report or an arrow.
- Known limit: a reply whose first read ends before its end (for instance right after `ESC ]`, or `ESC ] 99 ; i=1`) is
  not read as one; it comes out as Alt+] and typed characters, as before the patch. kitty writes a reply in one go,
  so this is rare. (Reads that fill the 1 KiB buffer are the exception: more is known to wait, so the OSC is held.)
- Unit tests: `osc_replies_become_events`, `alt_bracket_is_still_a_key`, `an_osc_split_across_reads_waits_for_its_end`,
  `a_stray_osc_gives_back_what_it_held`, `the_longest_reply_is_read_and_a_longer_one_given_back` (`parse.rs`);
  `Parser::advance` in `mio.rs`: `esc_bracket_and_a_key_is_alt_bracket_and_the_key`,
  `an_st_split_between_esc_and_backslash_is_one_osc`, `a_stray_osc_then_typing_loses_nothing`.
  ttyak ignores the event for now (`an_osc_reply_is_no_key_and_changes_nothing`).
- A unit test for the SGR sequences: `test_parse_csi_sgr_mouse_side_buttons` (buttons 8 to 11: down, release, drag;
  button 12 still fails).
- Upstream's colour tests (`src/style/types/colored.rs`) take turns on the process-wide colour switch (a test-only
  mutex, `colour_switch`), and the ANSI parse test turns colours on for itself: run in parallel, one test turned colours
  off while another formatted, so `test_parse_ansi_fg`/`_bg` failed about every other run (and with `NO_COLOR` set
  where they run, they could fail every time). Tests only; the library is as published.
- `#![allow(unused_parens)]` in `src/lib.rs`: upstream has one needless pair of parentheses
  (`terminal/sys/unix.rs`). cargo hides warnings of registry crates but shows those of path crates, so
  without it every ttyak build would warn. Left the code as published.
- Files: `.cargo-ok`, `.github/` and `.travis.yml` of the registry copy are left out, as for rtc-turn.
  `src/event.rs` and `parse.rs` keep their CRLF line endings, so a diff shows only the patch.
- Nothing else. The Windows backend never builds the new variants (its console events carry no side
  buttons, no OSC), so it compiles unchanged. ttyak's own tests are `the_mouses_side_buttons_go_back_and_forward` and
  `the_mouses_buttons_10_and_11_do_nothing`.

The crate is not a member of ttyak's workspace, so its own tests (which need dev-dependencies ttyak does
not have) are not run by `cargo test`; run them from a copy outside the workspace.

## When to drop it

When a crossterm release decodes SGR buttons 8 and 9 (with variants ttyak can match; adjust
`crates/ttyak/src/tui/event_loop.rs` if they are named differently). Then delete this folder and the
`[patch.crates-io]` entry. Also when ttyak moves to a `ratatui` / `ratatui-image` that needs a newer
crossterm: the patch then no longer applies (cargo warns that it is unused), so check whether that release
has the fix, and if not, vendor and patch the new version the same way.
