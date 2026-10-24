# ttyak 0.12.0 — attention, lighter CI, requirements: Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Notifications, their sound, the bell and the unread badge work as one system — a click lands on exactly what
was shown and brings the window forward wherever the platform allows — plus `ctl status`, `ctl check`, written
requirements and a CI that does less useless work.

**Architecture:** A pure `attention` module classifies each event once (nothing / unread / for you) and computes the
badge state; gates (pause, schedule, system DND, viewing) and the user's switches decide the outputs. A process-wide
`Notifier` delivers bubbles per conversation (replace, withdraw, click) over the best route — the terminal's OSC 99,
else the system notifier — and a `raise` module brings the window forward after a click. The tab title, the status
file and F6 read the same state.

**Tech Stack:** Rust (ratatui 0.30, vendored crossterm 0.29, notify-rust 4.18, tokio, serde, chrono), GitHub Actions.

**Spec:** `docs/superpowers/specs/2026-10-25-attention-and-ci-design.md` (approved by the user 2026-10-25). Read it
first; this plan argues from it and does not repeat its reasons.

## Global Constraints

- The Bash tool runs **bash**, not fish: `NAME=value`, never `set NAME value`; `set -euo pipefail` first; never a
  recursive chmod/chown/rm/mv/cp or `find -delete` on a path built from a variable (literal paths or `${VAR:?}` after
  checking).
- Never print tokens, cookies, codes, keys or passwords. Never quote real names, emails or Slack ids in PROGRESS, docs,
  commits or tests — say A, B.
- The `test` profile is the user's work workspace: never sign it in/out, open it or post there. Live checks use
  rslack-testbed: profile `default` = A, `--profile b` = B.
- `settings.toml` / `keys.toml` are the user's: back up and restore, or use a fake home (0700) in the scratchpad.
- Never build `target/release` (the user runs it); `cargo build` before every live check; no `cargo fmt`.
- Never capture mic/camera/screen; never play sound in tests except a notification sound the user agreed to for that
  check. `pgrep -x`/`pkill -x` only, never `-f`; never `pkill ttyak`.
- Read test windows with `kitty @ --to unix:@ttyak-test get-text`, no screenshots; close them after
  (`hyprctl dispatch closewindow class:ttyak-test`). Save and restore the user's clipboard around live checks.
- No desktop prompts from tests (gpg/keyring/polkit). **Live checks that move focus or switch the user's workspace
  need the user's word at that moment**, or run where they can't touch the user's screen (a headless/nested
  compositor). The macOS automation prompt needs the user at the Mac.
- Live waits ≤ ~10 minutes per task. Mac `macbook` and VM `win11-dev`: clean up, restore the VM's power state.
- Every user-visible word through `tr`; everything that waits is configurable in seconds (`settings.toml` + F6) except
  internal timings (stated per task, as Ruling V of the 0.11.0 plan). Match the surrounding code: short `///` lines
  saying what, its naming and idiom.
- Run the whole `cargo test` before every commit; never commit on red; never chain a scripted edit with `git commit`;
  commit with explicit paths; trailer `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
- PROGRESS.md: one row per task, id **AT1, AT2, …** in task order, above "Tests:", LIVE/UNIT/NOT honesty, the commit
  hash; update the "Tests:" counts line. README and F6 help for the task's own feature go into that task.
- Implementers never push, tag, release, or dispatch subagents; the controller pushes `master` after each reviewed task.
- Names fixed here: module `tui/attention.rs`, `tui/title.rs`, `tui/osc99.rs`, `tui/raise.rs`, `tui/system_dnd.rs`,
  crate-root `status_file.rs`, `check.rs`; settings under `[notifications]`: `desktop`, `sound`, `bell`, `title_mark`,
  `follow_system_dnd`, `sound_volume`.

## Review Focus

1. **The user presses Alt+]** (or Esc then `]`) — it must still be that key, never swallowed or held while ttyak waits
   for an OSC end, and an OSC reply split across two reads must still parse. → Task 6 tests.
2. **A terminal that never reports focus** (`FocusGained`/`FocusLost` never arrive): no bell, read marks and viewing
   behave as today (focused assumed). → Task 2 and Task 7 tests.
3. **A click on a bubble whose message is gone or whose workspace isn't on screen** (deleted, conversation left,
   another workspace): lands where today's `open_target` / the others' click path would, never panics, never INSERT.
   → Task 3 test.
4. **Workspace names with control or bidi characters, or very long ones**, in the title and the status line: stripped,
   the mark still first. → Task 5 and Task 14 tests.
5. **No notification server, or one without actions / sound** (headless box, SSH): nothing panics or blocks, the badge,
   title, status and bell still work, the "On this machine" rows say what's missing. → Task 7 and Task 13 tests.

---

### Task 1: Lighter CI

**Files:**
- Modify: `.github/workflows/ci.yml`, `.github/workflows/release.yml`

**Interfaces:** none (CI only).

- [ ] **Step 1: ci.yml**
  - top-level `concurrency: { group: ci-${{ github.ref }}, cancel-in-progress: true }`;
  - `paths-ignore` (push and pull_request) gains `"site/**"` and `"tools/site/**"`, comment updated (Pages tests them;
    the name check covers them at the next code push);
  - every `Swatinem/rust-cache@v2` gets `with: save-if: ${{ github.ref == 'refs/heads/master' }}`; the apt
    `actions/cache/save` step's `if:` adds `&& github.ref == 'refs/heads/master'`;
  - linux and macos: drop `- run: cargo build --locked` (keep `cargo test --locked`);
  - windows-cross: `cargo build --locked --target x86_64-pc-windows-gnu` (debug); the DLL check reads
    `target/x86_64-pc-windows-gnu/debug/ttyak.exe` (the static runtime is set for all builds in `.cargo/config.toml`).
- [ ] **Step 2: release.yml** — the build matrix's `Swatinem/rust-cache@v2` gets `save-if: false` (restore only);
  if the release's x86_64-windows build saves the apt cache, that save is dropped too. Comment: releases come further
  apart than GitHub's 7-day eviction.
- [ ] **Step 3: actionlint** — `podman run --rm -v "$PWD:/repo:ro" -w /repo docker.io/rhysd/actionlint:latest`
  Expected: no output, exit 0. Remove the image afterwards (`podman rmi docker.io/rhysd/actionlint:latest`).
- [ ] **Step 4: Commit** `ci: cancel superseded runs, skip site-only pushes, debug Windows build, only master saves caches`
- [ ] **Step 5 (controller, after push):** the CI run on this commit is green; record its job timings
  (`gh run view <id> --json jobs`) in the PROGRESS row against the earlier ~2.5–4 min. NOT until they happen: the
  cancel (next pair of quick pushes), the site-only skip (Task 15's site push), release `save-if` (the 0.12.0 run).

### Task 2: No read marks while the terminal is unfocused

**Files:**
- Modify: `crates/ttyak/src/tui/event_loop.rs` (App fields near `terminal_focused` :349; the read calls near :5272;
  `feed_tick` :2244; focus handling :5376)

**Interfaces:**
- Produces: `App::focus_reported: bool` (false until the first `FocusGained`/`FocusLost`);
  `fn reading(focus_reported: bool, focused: bool) -> bool` (free fn in event_loop.rs: `!focus_reported || focused`).
  Task 7 uses both.

- [ ] **Step 1: Failing tests** (event_loop tests, with `test_app()`):
```rust
#[test]
fn reads_only_while_focused() {
    assert!(reading(false, false), "no focus report yet: as today");
    assert!(reading(true, true));
    assert!(!reading(true, false));
}
#[test]
fn an_unfocused_terminal_leaves_new_messages_unread() {
    // test_app with conversation D1 loaded and on screen, an unread message in it;
    // app.focus_reported = true; app.terminal_focused = false; run the loop's read step
    // → app.ws.unread["D1"].has_unreads stays true and nothing is sent to client.mark;
    // then FocusGained through the event handler → the next read step marks it read.
}
```
- [ ] **Step 2: Run** `cargo test -p ttyak reads_only_while_focused an_unfocused_terminal` → FAIL (not defined).
- [ ] **Step 3: Implement** — set `focus_reported = true` on any focus event; guard `mark_current_read`,
  `mark_thread_read` and the Threads feed's `tick` read-marking (pass `shown && reading(..)`) with `reading(..)`.
  Explicit actions (Enter on an Activity item, `␣U`, mark read keys) are not guarded.
- [ ] **Step 4: Run** the whole `cargo test` → PASS.
- [ ] **Step 5: Live** (kitty `ttyak-test`, profile default = A): A on DM with B; focus another window (the user's
  word if it moves their focus, else a second kitty window on the same workspace); B posts via
  `cargo run -q -- --profile b ctl post …`; `ctl api` as B or the sidebar shows the DM still unread; focus back → read.
  Also confirm Slack's own client keeps an unfocused conversation unread (web client in a background tab, as B) —
  report what it does.
- [ ] **Step 6: Commit** `fix: no read marks while the terminal is unfocused`; README line where reading is described.

### Task 3: A click lands on what was shown, in NORMAL

**Files:**
- Modify: `crates/ttyak/src/tui/event_loop.rs` (`open_target` :2746, `open_in_app` :2630, `open_hit_from` :2409, the
  others' click path in `run`)

**Interfaces:**
- Consumes: `notify::Target { channel, ts, thread_ts }` (unchanged).
- Produces: `App::open_target(&mut self, target: notify::Target)` guarantees: the message `ts` selected; for a reply,
  the thread open with that reply selected; for a huddle (ts None) the conversation; `self.focus` is a NORMAL focus
  (`Focus::Messages` or `Focus::Thread`), the composer's draft kept.

- [ ] **Step 1: Failing tests**:
```rust
#[test]
fn a_click_never_leaves_insert_mode() {
    // test_app; focus = Focus::Composer with draft "half typed"; open_target(Target{ channel:"C1", ts:Some(m), thread_ts:None })
    assert!(matches!(app.focus, Focus::Messages)); assert_eq!(app.mode(), Mode::Normal);
    assert_eq!(app.composer.text(), "half typed");
}
#[test]
fn a_click_on_a_reply_opens_its_thread_at_that_reply() { /* thread == Some((C1, parent)), reply selected, Focus::Thread */ }
#[test]
fn a_click_on_a_deleted_message_lands_in_its_conversation() { /* ts not in timeline → conversation shown, no panic */ }
```
- [ ] **Step 2: Run** → FAIL where today's code differs (record which).
- [ ] **Step 3: Implement** the guarantees in `open_target` (after `open_in_app`, force a NORMAL focus).
- [ ] **Step 4: Run** the whole `cargo test` → PASS.
- [ ] **Step 5: Commit** `fix: a notification click lands on its message, never in INSERT`.

### Task 4: The attention module — decision, state, outputs

**Files:**
- Create: `crates/ttyak/src/tui/attention.rs` (+ `mod attention;` in `tui/mod.rs`)
- Modify: `crates/ttyak/src/tui/notify.rs` (rules move out; `notification` / `huddle_notification` become thin
  wrappers so their tests keep passing)

**Interfaces:**
- Consumes: `Workspace`, `Message`, `others::Badge`, `notify::Target`.
- Produces:
```rust
pub struct Key { pub workspace: usize, pub channel: String }           // one bubble per conversation
pub struct Note { pub key: Key, pub title: String, pub body: String, pub target: notify::Target }
pub enum Decision { Nothing, Unread, ForYou(Note) }
pub fn decide(ws: &Workspace, workspace: usize, channel: &str, m: &Message) -> Decision;
pub fn decide_huddle(ws: &Workspace, workspace: usize, channel: &str, user: &str, in_it: bool, join_key: &str) -> Decision;
#[derive(Clone, Copy, PartialEq, Eq, Debug)] pub enum State { Read, Unread, ForYou }
impl State { pub fn mark(self) -> char /* '○' '●' '◆' */; pub fn name(self) -> &'static str /* "read" "unread" "for_you" */ }
pub fn state(ws: &Workspace, others: &HashMap<usize, Badge>) -> State;
pub enum TermFocus { Focused, Unfocused, Unknown }
pub struct Gates { pub paused: bool, pub system_dnd: bool, pub viewing: bool, pub focus: TermFocus }
pub struct Switches { pub desktop: bool, pub sound: bool /* sound != "off" */, pub bell: bool }
#[derive(Debug, PartialEq, Default)] pub struct Outputs { pub bubble: bool, pub sound: bool, pub bell: bool }
pub fn outputs(d: &Decision, g: &Gates, s: &Switches) -> Outputs;
```
  `decide` = today's `notify::notification` rules **without** the `viewing` and `dnd_active` checks (they are gates now);
  `state`: ForYou if this workspace's DMs/mentions as Slack counts them (`total_unread().1` plus thread mentions /
  unread DMs — the sidebar's own numbers) or any `others` badge `attention > 0`; else Unread if `total_unread().0 > 0`,
  threads or Activity unread, or any other badge `unread`; else Read (muted never count — `total_unread`'s rule).

- [ ] **Step 1: Failing tests** in attention.rs:
```rust
#[test] fn marks() { assert_eq!((State::Read.mark(), State::Unread.mark(), State::ForYou.mark()), ('○', '●', '◆')); }
#[test] fn state_follows_slacks_counts() {
    // empty ws → Read; a channel has_unreads → Unread; a DM with mentions 1 → ForYou;
    // a muted channel with has_unreads → Read; another workspace's Badge{attention:0, unread:true} → Unread;
    // Badge{attention:2, ..} → ForYou
}
#[test] fn gates_hold_back_everything() {
    // ForYou + paused → Outputs::default(); + system_dnd → default; + viewing → default
}
#[test] fn switches_pick_the_outputs() {
    // desktop off, bell on, Unfocused → Outputs{bubble:false, sound:false, bell:true}   (no bubble, no sound)
    // desktop on, sound on, Focused → {true, true, false}
    // bell on, focus Unknown → bell false
    // Unread / Nothing → default whatever the switches
}
```
- [ ] **Step 2: Run** `cargo test -p ttyak attention` → FAIL.
- [ ] **Step 3: Implement**; `notify::notification(ws, channel, m, viewing, now)` keeps its signature = `decide` plus
  the two old checks, returning `(title, body)`; same for `huddle_notification`.
- [ ] **Step 4: Run** the whole `cargo test` → PASS (all of notify.rs's existing tests unchanged).
- [ ] **Step 5: Commit** `feat: one attention decision — nothing, unread, for you`.

### Task 5: Tab title `○ ttyak - <workspace>`

**Files:**
- Create: `crates/ttyak/src/tui/title.rs`
- Modify: `tui/terminal.rs` (enter/restore), `tui/event_loop.rs` (set after each draw when changed),
  `tui/settings.rs` (`Notifications.title_mark: bool`, default true), `tui/prefs.rs` + `prefs_help.rs`
  (row "Mark in the tab title" in Notifications), README

**Interfaces:**
- Consumes: `attention::State`, `attention::state`.
- Produces: `pub fn text(state: State, workspace: &str) -> String`; `pub struct Title` with
  `pub fn set(&mut self, out: &mut impl Write, text: &str) -> io::Result<()>` (writes only when changed; pushes the
  terminal's title once, XTWINOPS `CSI 22;0 t`, before the first write) and `pub fn restore(&self, out)` (`CSI 23;0 t`
  if pushed). `TerminalGuard` owns one `Title` so the panic hook and Drop restore it.

- [ ] **Step 1: Failing tests**:
```rust
#[test] fn the_mark_comes_first() { assert_eq!(text(State::Unread, "rslack-testbed"), "● ttyak - rslack-testbed"); }
#[test] fn names_are_cleaned() {
    assert_eq!(text(State::Read, "a\u{1b}]2;x\u{7}b\u{202e}c"), "○ ttyak - a]2;xbc");   // no controls, no bidi
    assert!(text(State::ForYou, &"w".repeat(500)).chars().count() <= 80);              // cut, mark kept
}
#[test] fn written_once_per_change() { /* set twice with the same text → one OSC 2 in the buffer; push before it */ }
#[test] fn off_leaves_the_title_alone() { /* title_mark=false → nothing written, nothing pushed, restore writes nothing */ }
```
- [ ] **Step 2: Run** → FAIL. **Step 3: Implement** (OSC 2 via crossterm `SetTitle`). **Step 4: Run** whole suite → PASS.
- [ ] **Step 5: Live** kitty `ttyak-test`: `kitty @ --to unix:@ttyak-test ls | jq '..|.title? // empty'` shows
  `○ ttyak - rslack-testbed`; B posts in a channel → `●`; B DMs A → `◆`; read both → `○`; quit → the old title back.
- [ ] **Step 6: Commit** `feat: tab title with a read/unread mark`; README (tmux needs `set-titles`).

### Task 6: OSC replies reach ttyak (vendored crossterm)

**Files:**
- Modify: `vendor/crossterm/src/event.rs` (`Event::Osc(String)`), `vendor/crossterm/src/event/sys/unix/parse.rs`,
  `vendor/crossterm/TTYAK-PATCH.md`; `crates/ttyak/src/tui/event_loop.rs` (ignore `Event::Osc` until Task 9 uses it)

**Interfaces:**
- Produces: `crossterm::event::Event::Osc(String)` — the payload between `ESC ]` and `BEL` / `ESC \`, without them.
  Parsing only when `ESC ]` is followed by an ASCII digit; `ESC ]` with nothing more available stays Alt+`]`.

- [ ] **Step 1: Failing tests** (ttyak, like `the_mouses_side_buttons_go_back_and_forward`; parse through the public
  API the existing test uses):
```rust
#[test] fn osc_replies_become_events() { /* b"\x1b]99;i=w0-C1;\x1b\\" → Event::Osc("99;i=w0-C1;"); same with \x07 */ }
#[test] fn alt_bracket_is_still_a_key() { /* b"\x1b]" with no more input → Key(']' + ALT) */ }
#[test] fn an_osc_split_across_reads_waits_for_its_end() { /* b"\x1b]99;i=w0" with more input → None, then whole → Osc */ }
```
- [ ] **Step 2: Run** → FAIL. **Step 3: Implement**, every changed line marked `// ttyak:`, CRLF kept; TTYAK-PATCH.md
  gets the why and the list. **Step 4: Run** whole suite → PASS.
- [ ] **Step 5: Live** kitty: Alt+] in NORMAL and in INSERT behaves as before (nothing typed, nothing stuck).
- [ ] **Step 6: Commit** `vendor: crossterm parses OSC replies (for kitty's notification clicks)`.

### Task 7: The Notifier — one bubble per conversation, withdraw on read, bell (system notifier)

**Files:**
- Modify: `tui/notify.rs` (a process-wide `Notifier`), `tui/event_loop.rs` (create in `run`, pass to
  `run_workspace`; the incoming-message / huddle paths at :6503–6545 call `attention` + `Notifier`; withdraw on read),
  `tui/others.rs` (watchers use the same `Notifier`, key `workspace: index`), `tui/settings.rs` (`bell: bool` default
  true), `prefs.rs`/`prefs_help.rs` (row "Bell when not focused"), README

**Interfaces:**
- Consumes: `attention::{decide, decide_huddle, outputs, Key, Note, Gates, Switches, TermFocus}`;
  `App::focus_reported` (Task 2).
- Produces:
```rust
#[derive(Clone)] pub struct Notifier { /* Arc<Mutex<..>>: live bubbles by Key → server id + click waiter */ }
impl Notifier {
    pub fn new() -> Notifier;
    pub fn show(&self, note: Note, sound: Option<String>, on_click: Box<dyn FnOnce() + Send>);  // replaces the Key's bubble
    pub fn withdraw(&self, key: &Key);                                                          // closes it, ends its waiter
    pub fn show_silent(&self, title: String, body: String, on_click: Option<Box<dyn FnOnce() + Send>>); // the update notice
    pub fn route(&self) -> RouteInfo;   // for F6's "On this machine" (Task 13)
}
pub struct RouteInfo { pub shown_by: String, pub sound_by: String, pub click: String }  // Tasks 8–12 fill their parts
pub fn bell(out: &mut impl Write) -> io::Result<()>;   // writes "\x07"
```
  freedesktop: `replaces_id` from the Key's live id; `CloseNotification` on withdraw; one waiter per id (a replaced
  bubble's waiter ends). Withdraw when: `mark_current_read`/`mark_thread_read` mark that conversation, a Slack read
  event (`channel_marked`/`im_marked`/`thread_marked` in `live::apply`'s effects) arrives for it, or the others' poll
  shows that workspace without unreads for it (best effort, 30 s). Sound stays ttyak's player for now (Task 8 moves it).

- [ ] **Step 1: Failing tests**:
```rust
#[test] fn a_newer_message_replaces_the_bubble() { /* fake backend: show(K,1) then show(K,2) → second call carries replaces_id of the first */ }
#[test] fn reading_withdraws_it() { /* show(K) then withdraw(K) → close(id) once; withdraw again → nothing */ }
#[test] fn no_server_no_panic() { /* backend returns Err → show logs, withdraw no-op, bell still written */ }
#[test] fn the_bell_rings_only_unfocused_and_reported() { /* App: ForYou, focus_reported=false → no BEL; reported+unfocused → one BEL */ }
```
  (The backend sits behind a small private trait so tests use a fake; the real one is notify-rust.)
- [ ] **Step 2: Run** → FAIL. **Step 3: Implement**. **Step 4: Run** whole suite → PASS.
- [ ] **Step 5: Live** (swaync here): B sends A three DMs while ttyak is unfocused → one bubble, showing the third;
  A reads on another client (B can't; use `ctl api conversations.mark` as A from a shell) → the bubble disappears;
  `kitty @ ls` / Hyprland shows the urgent hint after the bell (`hyprctl clients -j | jq '.[]|select(.class=="ttyak-test")|.urgent'`
  if exposed, else the user's eye — ask).
- [ ] **Step 6: Commit** `feat: one notification per conversation, withdrawn when read; the bell`.

### Task 8: The sound goes with the notification; the system's do-not-disturb (Linux)

**Files:**
- Create: `crates/ttyak/src/tui/system_dnd.rs`
- Modify: `tui/notify.rs` (sound plan per route), `tui/sound.rs` (`pub fn wav_file(name: &str) -> Option<PathBuf>`:
  the synthesized sound written once as WAV into the cache dir), `tui/settings.rs` (`follow_system_dnd: bool` default
  true), prefs rows, README

**Interfaces:**
- Produces:
```rust
pub enum SoundBy { Notification, Ttyak, None }
pub fn sound_by(capabilities: &[String], sound: &str) -> SoundBy;  // "sound" capability → Notification; "off" → None
pub enum Server { Swaync, Mako, Dunst, Gnome, Kde, Other(String) }
pub fn server(name: &str) -> Server;                               // from GetServerInformation's name
pub async fn suppressed(server: &Server) -> Option<bool>;          // None: can't tell
```
  `suppressed`: swaync → D-Bus `org.erikreider.swaync.cc` `GetDnd`; mako → `makoctl mode` lists `do-not-disturb`;
  dunst → `dunstctl is-paused`; GNOME → `org.gnome.desktop.notifications show-banners` false; KDE →
  `org.freedesktop.Notifications` property `Inhibited`. Asked per for-you event with a 1 s internal timeout (a
  constant: an internal wait, not a user-facing one); None or `follow_system_dnd = false` → `Gates.system_dnd = false`.
  When the server plays the sound: `sound-file` = `wav_file(name)`, `system` → `sound-name`; ttyak's volume and output
  device then don't apply (F6 help says so). Sound `off` → `suppress-sound` hint on every route.

- [ ] **Step 1: Failing tests**: `sound_by` table (`["sound"]` + chime → Notification; `[]` + chime → Ttyak; any +
  off → None); `server("SwayNotificationCenter")` → Swaync, `"mako"`, `"dunst"`, `"gnome-shell"`/`"GNOME Shell"`,
  `"Plasma"`/`"KDE"`; `off_mutes_the_server` (hints contain `SuppressSound(true)`); `wav_file("chime")` exists and
  starts with `RIFF`, `wav_file("off")` None.
- [ ] **Step 2: Run** → FAIL. **Step 3: Implement**. **Step 4: Run** whole suite → PASS.
- [ ] **Step 5: Live** (the user agrees to hear a chime first): swaync DND on (`swaync-client -dn`) → a DM from B
  shows no bubble and no sound; DND off (`swaync-client -df`) → bubble + chime. Restore the user's DND state as found.
- [ ] **Step 6: Commit** `feat: notification sounds follow the system's do-not-disturb`.

### Task 9: Notifications through the terminal (OSC 99)

**Files:**
- Create: `crates/ttyak/src/tui/osc99.rs`
- Modify: `tui/media.rs` (startup query: add the OSC 99 probe before the DA1 fence it already sends), `tui/notify.rs`
  (route `Osc99`), `tui/event_loop.rs` (`Event::Osc` → click / close reports), README

**Interfaces:**
- Consumes: `Event::Osc(String)` (Task 6); `Notifier` (Task 7).
- Produces:
```rust
pub fn probe() -> &'static str;                         // "\x1b]99;i=ttyak-probe:p=?;\x1b\\"
pub fn supported(reply: &str) -> bool;                  // a "99;i=ttyak-probe:p=?;…" answer listing a=focus,report and p=title,body
pub fn id(key: &Key) -> String;                         // "w<index>-<channel>", only [A-Za-z0-9_-]
pub fn show(id: &str, title: &str, body: &str, sound: Option<&str>) -> String;  // a=focus,report; o=unfocused only if not viewing; e=1 base64 payloads; s=
pub fn close(id: &str) -> String;                       // p=close
pub fn wrap_for_tmux(seq: &str) -> String;              // ESC P tmux; … ESC \ with ESC doubled
pub enum Report { Clicked(String), Closed(String) }
pub fn report(osc: &str) -> Option<Report>;             // "99;i=<id>;" → Clicked; c=1 close → Closed
```
  Under tmux: Claude's call for the builder to confirm — use OSC 99 only when `tmux show -gv allow-passthrough` is
  `on`/`all` **and** ttyak's pane is the active one at send time; otherwise the system notifier (reports go to the
  active pane). The Notifier keeps a Key ↔ id map so a report finds its target; withdraw sends `close`.

- [ ] **Step 1: Failing tests**: `id` (`w0-C0123` from Key{0,"C0123"}; odd characters dropped); `show` round-trips
  title/body through base64 (`e=1`), contains `a=focus,report`; `report("99;i=w0-C1;")` → Clicked("w0-C1");
  `supported` true for kitty 0.48's reply, false for empty/garbage; `wrap_for_tmux` doubles ESC.
- [ ] **Step 2: Run** → FAIL. **Step 3: Implement**. **Step 4: Run** whole suite → PASS.
- [ ] **Step 5: Live** kitty `ttyak-test` (focus-moving → the user's word first): ttyak unfocused on another
  workspace; B DMs A → kitty's bubble; the user clicks → kitty focuses the window+tab, ttyak lands on the message in
  NORMAL. Whether swaync 0.9.0 (no ActivationToken) lets kitty cross workspaces: report. A second DM replaces the
  bubble; reading closes it.
- [ ] **Step 6: Commit** `feat: notifications through the terminal (kitty's OSC 99)`.

### Task 10: Bring the window forward after a click (Linux, X11, tmux, kitty, WezTerm)

**Files:**
- Create: `crates/ttyak/src/tui/raise.rs`
- Modify: `tui/event_loop.rs` (`open_target` calls `raise::raise` for system-notifier clicks — also a click from
  another workspace's bubble, after the switch), README (what works
  where, the can't-do list from the spec)

**Interfaces:**
- Produces:
```rust
pub struct Env { pub vars: HashMap<String, String>, pub ancestors: Vec<u32> /* ttyak's parent pids, nearest first */, pub os: &'static str }
pub fn env() -> Env;
pub enum Raise { Tmux { pane: String, then: Box<Raise> }, Kitty { window: String }, Wezterm { pane: String },
                 Hyprland { pid: u32 }, Sway { pid: u32 }, Niri { pid: u32 }, Kwin { pid: u32 }, X11 { window: u64 },
                 Mac(MacRaise), Windows, None }
pub fn detect(env: &Env) -> Raise;           // pure: order below
pub fn raise(r: &Raise);                     // runs it; each command bounded by a 2 s internal timeout; failures logged
pub fn describe(r: &Raise) -> String;        // "via Hyprland" … for F6 (through tr)
```
  Order (spec): tmux (`TMUX`, `TMUX_PANE`; then the outer terminal from `#{client_pid}`) → kitty (`KITTY_WINDOW_ID` and
  remote control answering `kitty @ ls`) → WezTerm (`WEZTERM_PANE`) → Hyprland (`HYPRLAND_INSTANCE_SIGNATURE`; the pid
  is the ancestor that `hyprctl -j clients` lists) → sway (`SWAYSOCK`) → niri (`NIRI_SOCKET`) → KDE (`KDE_SESSION_VERSION`
  → KWin script over D-Bus) → X11 (`DISPLAY` without `WAYLAND_DISPLAY`: `WINDOWID` else `_NET_WM_PID` of an ancestor;
  EWMH `_NET_ACTIVE_WINDOW` with source 2) → None. kitty raise and the WM raise may both run (kitty picks the tab, the
  WM switches the workspace) — the builder decides from the live check.

- [ ] **Step 1: Failing tests** (`detect` on hand-built `Env`s): kitty on Hyprland → Kitty then Hyprland chosen as
  expected; tmux inside kitty → Tmux{then: Kitty}; sway; niri; X11 with WINDOWID; GNOME Wayland (only
  `XDG_CURRENT_DESKTOP=GNOME`, `WAYLAND_DISPLAY`) → None; `describe(None)` says the window can't be raised.
- [ ] **Step 2: Run** → FAIL. **Step 3: Implement**. **Step 4: Run** whole suite → PASS.
- [ ] **Step 5: Live**: here with the user's word (Hyprland + kitty, from another workspace, via the system notifier:
  force the System route with a debug env var if OSC 99 would take it — `TTYAK_NOTIFY_ROUTE=system`, documented as
  debug); sway and X11 in a headless/nested session (`sway` with `WLR_BACKENDS=headless`, `Xvfb` + a WM) when
  available without installing packages on the host (podman otherwise); tmux in a kitty window.
- [ ] **Step 6: Commit** `feat: a click brings the terminal window forward`.

### Task 11: macOS — clicks, the host app, Terminal.app / iTerm2 tabs

**Files:**
- Modify: `tui/notify.rs` (macOS: click callback on a blocking thread; `set_application(<host bundle id>)`),
  `tui/raise.rs` (`MacRaise { Iterm2 { session: String }, Terminal { tty: String }, App { bundle: String } }`),
  `tui/system_dnd.rs` (macOS Focus only if an unprivileged way exists — else None), README

**Interfaces:**
- Consumes: `Raise`, `Notifier`.
- Produces: `fn host_bundle(vars: &HashMap<String,String>) -> Option<&'static str>` (`TERM_PROGRAM`: `Apple_Terminal` →
  `com.apple.Terminal`, `iTerm.app` → `com.googlecode.iterm2`, `WezTerm` → `com.github.wez.wezterm`, `ghostty` →
  `com.mitchellh.ghostty`; `KITTY_WINDOW_ID` → `net.kovidgoyal.kitty`).

- [ ] **Step 1: Failing tests**: `host_bundle` table; `detect` on macOS Envs → Mac(Iterm2{ITERM_SESSION_ID}) /
  Mac(Terminal{tty}) / Mac(App{..}); the AppleScript text for Terminal selects the tab whose `tty` matches.
- [ ] **Step 2: Run** → FAIL. **Step 3: Implement** (no .app bundle, no packaging change — the user's anti-goal).
  **Step 4: Run** whole suite → PASS (on the Mac too: `ssh macbook` build + `cargo test`).
- [ ] **Step 5: Live** on `macbook` with the user at it: Terminal.app and iTerm2 (if installed), ttyak in a background
  tab, B DMs A → click → the right app, window and tab, the message, NORMAL; the one-time automation prompt seen by
  the user. Whether the click reaches ttyak at all from an unbundled binary: report; if not, the click activates the
  host app (set_application) and the conversation is already open — say so in README. Clean up.
- [ ] **Step 6: Commit** `feat: macOS notification clicks bring the terminal tab forward`.

### Task 12: Windows — toast clicks, replace by tag, console window forward, Focus Assist

**Files:**
- Modify: `tui/notify.rs` (Windows: activation callback; replace/withdraw via toast `Tag`/`Group` — notify-rust ignores
  `id` on Windows: patch it under `vendor/` with a TTYAK-PATCH.md, or call the `windows` crate directly — the smaller
  one), `tui/raise.rs` (`Windows`: `GetConsoleWindow` → its root owner → `SetForegroundWindow`, best effort),
  `tui/system_dnd.rs` (`SHQueryUserNotificationState`: busy/quiet time/presentation → suppressed), README

**Interfaces:** as Tasks 7, 8, 10 (same `Notifier`, `Raise::Windows`, `suppressed`).

- [ ] **Step 1: Failing tests** (cfg(windows), run on the VM): the toast XML/tag for Key{0,"C1"} is stable; withdraw
  removes by that tag; the QUNS value mapping.
- [ ] **Step 2: Run on `win11-dev`** → FAIL. **Step 3: Implement**. **Step 4: Run** whole suite on Linux and the VM → PASS.
- [ ] **Step 5: Live** on the VM (Windows Terminal and conhost): ttyak in the background, B DMs A → toast; click →
  window forward (WT: whatever tab is current — known can't-do), message, NORMAL; second DM replaces; reading
  withdraws; Focus Assist on → nothing. Restore the VM's state.
- [ ] **Step 6: Commit** `feat: Windows notification clicks, replacing toasts, Focus Assist`.

### Task 13: F6 → Notifications as the one place

**Files:**
- Modify: `tui/prefs.rs` (`CATEGORIES` "Notifications", new `Field`s), `tui/prefs_help.rs`, `tui/settings.rs`
  (`Notifications.sound_volume`; `[audio] sound_volume` still read when the new key is absent — serde default from
  the old value at load), `tui/event_loop.rs` (test notification), README's F6 row

**Interfaces:**
- Consumes: `Notifier::route()`, `raise::describe`, `system_dnd::server`/`suppressed`, `osc99::supported`.
- Produces: Notifications fields in this order: `ConversationLevel, ChannelDefault, Keywords, Vips` · `Desktop, Sound,
  SoundVolume (moved from Audio & video), Bell, TitleMark` · `PausedUntil (same as Availability's Notifications row),
  Schedule, FollowSystemDnd` · `ShownBy, ClickResult, SystemDnd, TestNotification` (the last four shown-only /
  action rows like `UpdateVersion` / `UpdateCheckNow`). If F6 has no sub-headings, the order carries the grouping.

- [ ] **Step 1: Failing tests**: the category lists exactly those fields; `SoundVolume` no longer in "Audio & video";
  an old `settings.toml` with `[audio] sound_volume = 40` and no `[notifications] sound_volume` loads 40; the
  shown-only rows ignore Enter/toggle; `TestNotification` Enter sends a ForYou-like note through the Notifier (fake)
  and, after 3 s (constant: it is part of the test action), rings the bell if unfocused; every new row has help and
  its words are in the locale template (`help_is_translatable`).
- [ ] **Step 2: Run** → FAIL. **Step 3: Implement**. **Step 4: Run** whole suite → PASS.
- [ ] **Step 5: Live** kitty: F6 → Notifications shows the rows with this machine's values (kitty / ttyak plays the
  sound / via kitty / swaync: off); Send a test notification works.
- [ ] **Step 6: Commit** `feat: F6 → Notifications holds the whole notification system`.

### Task 14: `ttyak ctl status`

**Files:**
- Create: `crates/ttyak/src/status_file.rs` (+ `mod status_file;` in main.rs)
- Modify: `crates/ttyak/src/ctl.rs` (`Ctl::Status`), `tui/event_loop.rs` (write on change, remove on quit), README

**Interfaces:**
- Consumes: `attention::state`, `others::Badges`, `ws.dnd`, `App::call`, `update.available`.
- Produces:
```rust
#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Status { pub version: u32 /* 1 */, pub running: bool, pub pid: u32, pub profile: String, pub state: String,
    pub mark: String, pub for_you: u64, pub workspaces: Vec<WorkspaceStatus>, pub paused_until: Option<String>,
    pub huddle: Option<HuddleStatus>, pub update: Option<String>, pub updated_at: String }
pub struct WorkspaceStatus { pub name: String, pub current: bool, pub unread: bool, pub for_you: u64 }
pub struct HuddleStatus { #[serde(rename = "where")] pub place: String }
pub fn path(profile: &str) -> PathBuf;     // $XDG_RUNTIME_DIR/ttyak/<profile>.json; else <cache>/<profile>/status.json
pub fn write(profile: &str, s: &Status) -> io::Result<()>;   // 0600, temp + rename
pub fn remove(profile: &str);
pub fn read(profile: &str) -> Status;      // missing, unreadable or pid not alive → running:false
pub fn line(s: &Status) -> String;         // "◆ rslack-testbed: 2 for you · paused until 14:00" / "ttyak is not running"
Ctl::Status { #[arg(long)] json: bool, #[arg(long)] follow: bool, #[arg(long, default_value_t = 1)] every: u64 }
```
  Dates RFC 3339 with offset (chrono). `--follow` prints when the file's content changes, checking every `--every`
  seconds; exit 0 when not running. Handled in `ctl::run` before a client is needed. Never message text or authors.

- [ ] **Step 1: Failing tests**: `line` for read/unread/for-you/paused/not-running and a name with controls (cleaned);
  JSON round-trip with the exact field names (`"where"`); `read` of a file whose pid is gone → `running:false`;
  `write` makes mode 0600 (unix); `path` with and without `XDG_RUNTIME_DIR` (set on a test env, not the process).
- [ ] **Step 2: Run** → FAIL. **Step 3: Implement**. **Step 4: Run** whole suite → PASS.
- [ ] **Step 5: Live**: ttyak running on A → `cargo run -q -- ctl status`, `--json | jq .state`, `--follow` while B
  DMs A (a new line appears), quit → "not running". README: two `jq` examples (a plain bar line; the mark only).
- [ ] **Step 6: Commit** `feat: ttyak ctl status for bars and scripts`.

### Task 15: Requirements and `ttyak ctl check`

**Files:**
- Create: `crates/ttyak/src/check.rs`
- Modify: `crates/ttyak/src/ctl.rs` (`Ctl::Check`), README (a **Requirements** section, per system: needs / works
  best with — the spec's list), the site's install section (mirror per `tools/site` conventions; run
  `python3 tools/site/check_private.py site` before the site commit)

**Interfaces:**
- Consumes: `osc99::supported`, `raise::{env, detect, describe}`, `system_dnd::server`, the media probe's results.
- Produces: `pub struct Findings { terminal: Vec<(String, Option<bool>)>, system: Vec<(String, Option<String>)> }`;
  `pub fn findings() -> Findings` (queries the terminal in raw mode briefly, like media.rs; no network, no Slack);
  `pub fn report(f: &Findings) -> String`; `pub const GLYPHS: &[(&str, &str)]` — symbols `● ○ ◆ ★ ⏎ ↪ ⬆ ✕`, arrows,
  box/blocks `─│┌┘▀▄█░`, emoji `👀 🎧 📎 🎉`, CJK `日本語 中文 한국어`.

- [ ] **Step 1: Failing tests**: `report` lists every glyph group line and says "Look for empty boxes or misaligned
  columns"; unknown values print "not known"; all words through `tr`.
- [ ] **Step 2: Run** → FAIL. **Step 3: Implement**. **Step 4: Run** whole suite + site suites
  (`python3 -m unittest discover -s tools/site -p 'test_*.py'`, `node --test "tools/site/*.test.mjs"`) → PASS.
- [ ] **Step 5: Live**: `ctl check` in kitty here, on the Mac (Terminal.app) and the VM (Windows Terminal and conhost);
  the README's Terminal.app colour line from what the Mac showed.
- [ ] **Step 6: Commit** `feat: ttyak ctl check, and the requirements written down`.

### Task 16 (controller): final review, release prep

- [ ] Whole-branch review since `v0.11.0` (split app / CI+docs as at 0.11.0). Fix blockers only before the tag; offer
  the user "blockers, then tag" (release-day lessons).
- [ ] Bump to 0.12.0, whole `cargo test`, push, CI green → ask the user for the tag. Release notes by hand.
- [ ] Add the navigation rework to PROGRESS NEXT UP with the spec's section 6 notes.
