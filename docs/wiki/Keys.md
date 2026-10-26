The keys ttyak uses by default. Press **F1** (or `?` in the sidebar, the messages and the views) in the app for the full
list with the keys in effect, your own bindings included. `ttyak ctl keys-template` prints every action with its
default keys (see [Scripting](Scripting)). To change a key, see [Rebinding keys](Rebinding-keys).

## Modes and the leader

ttyak has modes, like vim:

- **NORMAL** — the sidebar, the messages, a thread, the views. Only moving around, help, preferences and Y/N questions
  sit on a key of their own; everything else — anything that changes something — is behind the **leader**, `Space`
  (written `␣` here and in the app).
- **INSERT** — typing in the composer or the reply box. `i` (or Enter in the sidebar) starts it, Esc leaves it.
- **DIALOG** — jump to, search, preferences, menus, pickers, the profile card, the image viewer, … The dialog's keys
  are shown at the bottom.

The pill at the left of the status line says which mode you are in ([The screen](The-screen#the-status-line)). While
you type, or in a dialog, Space is a space, so the leader there is **`Ctrl+Space`**.

Press the leader and wait (0.4 s) and the **which-key** panel opens in a corner with what each next key does; a wrong
key or Esc cancels, Backspace goes back a key. It follows where you are: on a message it lists the message's actions,
on a conversation the conversation's. Whether it shows, its corner, delay, height and icons are in F6 → Keys.

Ctrl+Space is also what macOS uses to switch input sources, and Windows input methods may take it. If ttyak never sees
it, rebind `insert_leader` in F6 → Keys.

## Everywhere

| Key | |
|---|---|
| ↑ ↓ / `k` `j`, Home End / `g` `G`, PgUp PgDn, Ctrl+U / Ctrl+D | move, first / last, a page, half a page — in the conversation, the sidebar, the Threads, Activity and Huddles views, the key list (F1) and a snippet (there: lines, start / end, a page, half a page) |
| Tab / Shift+Tab | move between sidebar, messages, composer, thread, reply box |
| Alt+↑ / Alt+↓ | previous / next conversation |
| Alt+Shift+↑ / Alt+Shift+↓ | previous / next unread conversation (muted ones are skipped) |
| Ctrl+O · Alt+← / Alt+→ | back / forward, as in a browser ([below](#back-and-forward)) |
| Ctrl+K · `␣␣` · `␣ff` | jump to any conversation or person ([below](#jump-to)) |
| Ctrl+F · `␣fs` | search messages, the newest match first (more results load as you move down the list) |
| Ctrl+T · `␣ft` | the [Threads view](The-screen#the-threads-view) |
| Ctrl+A · `␣fa` | the [Activity view](The-screen#the-activity-view) |
| `␣fh` | the [Huddles view](The-screen#the-huddles-view) |
| `␣fc` | every channel (as Ctrl+K with `#`) |
| F6 · `/prefs` | preferences: Availability, Appearance, Messages & media, Language & region, Notifications (the whole notification system: what notifies you, how you are told, quiet hours, what this machine does, a test notification), Audio & video, Huddles, Updates, Keys ([Settings](Settings), [Notifications and attention](Notifications-and-attention#settings-in-f6)) |
| F1 · `?` | all keys |
| Alt+1 … Alt+9 · `␣w1` … `␣w9` | the profile's workspace 1 … 9 |
| Ctrl+Q | quit |
| Enter (on a message) · `␣t` | open its thread |
| Enter · `i` (sidebar) | type in the conversation |
| `i` (messages, thread) | type (in a thread: reply) |
| Esc | leave the composer · close the thread or a dialog · back from the Threads / Activity / Huddles view (also `q` there) |
| mouse | [below](#the-mouse) |

### Jump to

Ctrl+K lists every conversation and person: unread first, then the most recently active. With something typed, a name
equal to it comes first (ignoring case, spaces and a leading `#` / `@`; a person's display name, real name or handle),
then the names starting with it, then the rest. `#` lists every channel, including ones you are not in.

### Back and forward

Ctrl+O or Alt+← goes back, Alt+→ forward, as in a browser: to the conversation (its thread open again if it was), the
Threads, Activity (on its tab) or Huddles view you came from, at the message you were on. In detail:

- It goes back to the reply selected in the open thread, or the item selected in the Threads or Activity view (their
  next pages loaded to find it, up to five, until you do something there or leave), loading older history until the
  message is there, as for a link, until you do something there. A message deleted meanwhile: the one before it, said
  so. A selection you had left behind — sending a message takes you to the newest — or the newest message itself: at
  the newest, following new ones.
- It counts however you went there: the sidebar, Ctrl+K, a link, Enter in a view, a notification. Moving through the
  sidebar — ↑/↓, the wheel over it, Alt+↑/↓ — is one step however far, back to where you were before it.
- The last 20 places are kept, per workspace (kept while you are in another). Going somewhere new after going back
  drops the places ahead. A channel left or archived meanwhile is skipped.
- Only in NORMAL, never while you type.
- The mouse's side buttons do the same (button 8 back, 9 forward), in a terminal that reports them; ttyak reads them
  through a patched `crossterm` (`vendor/crossterm`).

### The mouse

- Click to select; the wheel scrolls (also the help and the preferences). In lists like Ctrl+K, search and the
  Activity and Huddles views the wheel moves the selection; it scrolls the Threads feed.
- A selection that a scroll — the wheel, PgUp / PgDn — leaves off screen goes onto the nearest message still there:
  the top one after scrolling down, the bottom one after scrolling up, so ↑ / ↓ go on from what you see. With nothing
  selected, a scroll up selects in the pane with the focus.
- New messages are not a scroll: a selection they push off the top of a conversation at its newest stays on the
  message you picked (the keys still act on it), the conversation still at its newest. A resize, or a thread opened
  beside, keeps a conversation at its newest, and brings the selection back into view in one scrolled up and in the
  views.
- Hover shading covers the whole message row, edge to edge. Right-click opens a context menu (messages, sidebar
  entries).
- A click on a tile of the workspace rail switches workspace; a click on a sidebar section's header folds it.

## On a message

After `Space` (written `␣`), on a **message**:

| Key | |
|---|---|
| `␣r` `␣e` `␣d` | react · edit · delete (the reaction picker also shows who reacted: `remove · you, alice`) |
| `␣y` `␣Y` | copy the text · copy a link to the message |
| `␣o` | open what the message holds — a link (one into your Slack opens in ttyak, others in your browser), a picture, a video, a snippet; with several, a list to pick from ([Images and video](Images-and-video), [Links into your Slack](Links-into-your-Slack)) |
| `␣s` | save files (a dialog: Space / `a` pick which, Tab changes the folder for this save, Enter saves) |
| `␣p` | pin / unpin |
| `␣u` | the author's profile card ([below](#the-profile-card)) |
| `␣m` | the message's menu, as a right-click opens it |
| `␣U` | mark unread from this message (the conversation is unread again from here; ttyak leaves it so until you switch away and back) |
| `␣hT` | start (or join) a huddle in the message's thread |

**Reaction pills.** A click on a reaction pill under a message — in a conversation, a thread, the Threads feed or
Activity (there also the pill of a "Reacted to" item), a custom emoji's picture too — adds your reaction with that
emoji, or takes it off when it is yours, as in Slack. The message is selected (an Activity item is not opened) and the
pill changes at once; Slack's own event for it changes nothing more. A second click before Slack has answered the
first is ignored (a double-click is one), and one Slack refuses (an archived channel, no connection) is put back with
Slack's reason in the status line — from the `␣r` picker too. A click beside a pill does what it did. The pointer over
a pill says who reacted in the status line (`:eyes: · you, alice, +2`).

The same keys work on the selected message of the Threads feed and on an Activity item that is a message, the view
staying (`␣t` says that Enter opens it there). They are the `[messages]` keys, so rebinding one there rebinds it in
those views too; their own keys come first (a `[threads]` or `[activity]` binding on the same keys wins there).

### The profile card

`␣u` shows the author's big profile picture on top (as Slack shows it; smaller on a short terminal, none when too
short; their initials while it loads and where the terminal draws no pictures), then name, status, local time and the
rest. On the card (a dialog, so its leader is Ctrl+Space):

| Key | |
|---|---|
| ← / → (Shift+Tab / Tab) | the people the message mentions |
| Enter · `m` | message them |
| `Ctrl+Space h p` | start a huddle with them |
| `Ctrl+Space v` | add them to your VIPs, or take them off |
| Esc · `q` | close |

## On a conversation

On a **conversation** in the sidebar:

| Key | |
|---|---|
| `␣s` | star / unstar |
| `␣m` | move to a section (or a new one) |
| `␣d` | details: about, members, pins |
| `␣b` | what it notifies about (the next level; see [Notifications and attention](Notifications-and-attention#desktop-notifications)) |
| `␣M` | mute / unmute |
| `␣y` | copy the conversation's link |
| `␣l` | leave (asks first) |

The same, and more, from anywhere under `␣c`: `␣cs` `␣cm` `␣cd` `␣cb` `␣cM` `␣cy` `␣cl`, and `␣cu` mark unread from
the selected message or the latest, `␣cj` join the channel you are looking at, `␣cr` rename and `␣cx` delete the
conversation's section. The sidebar's right-click menu offers mute, mark unread and copy link too.

## The other groups

| Group | |
|---|---|
| `␣h` huddle | `h` join or leave · `m` mute · `t` its thread · `v` huddle view · `c` camera · `s` share screen · `d` decline an invitation ([Huddles](Huddles)) |
| `␣n` new | `c` channel · `p` private channel · `d` DM · `s` snippet · `f` upload a file |
| `␣a` availability | `s` set status · `c` clear it · `a` away / active · `n` pause notifications for 30 minutes, or resume them when paused · `N` pause for a time you type (`/dnd `) |
| `␣f` find | `f` jump to · `s` search · `a` activity · `t` threads · `h` huddles · `c` every channel |
| `␣w` workspace | `1` … `9` |

The huddle view's own keys (↑ ↓ ← → or `h` `j` `k` `l` between tiles, Enter or `f` to maximize a tile, Esc for all
tiles again or to leave them) work while it has the focus.

## Sections in the sidebar

In the sidebar `z` (or a click on a header) folds / unfolds the section. Folded, only "▸ name" shows, with the number
of its conversations with unread messages and a badge for mentions. Enter on it unfolds it too, and going to a
conversation inside (Ctrl+K, search, a notification) unfolds it. The folds are kept per workspace account in
`sidebar/<team>-<user>.json` under the local data folder (below).

`/section rename <name>` and `/section delete` (or `␣cr`, `␣cx`) rename and delete the section of the current
conversation; `␣m` moves a conversation to another.

## Drafts

What you type is kept per conversation and per thread: leave a half-written message (or an edit in progress) and it is
back when you return, until you send it. The sidebar marks such conversations with ✎.

Drafts also survive quitting, switching workspace and crashes: they are saved two seconds after you stop typing, per
workspace account, in `drafts/<team>-<user>.json` under the local data folder (Linux `~/.local/share/ttyak`, or under
`$XDG_DATA_HOME`; macOS `~/Library/Application Support/ttyak`; Windows `%LOCALAPPDATA%\ttyak\data`). Only you can
read the file (from 0.12.2 on, on Windows too). It goes once no draft is left. Attachments stay with the running
ttyak: a draft of only attachments is not saved.

## Typing (INSERT)

| Key | |
|---|---|
| Enter · Alt+Enter (also Shift+Enter, Ctrl+J) | send · new line |
| Esc | cancel an edit, else leave the box (back to NORMAL) |
| ↑ in an empty box | edit your last message |
| Ctrl+V | paste from the clipboard: text is typed in; a picture (a screenshot) or files copied in a file manager are attached above the box (✕, or Backspace in an empty box, removes one) and Enter sends them with your text — or on their own — as one message. Your terminal's own paste (Ctrl+Shift+V) only carries text, and over SSH there is no clipboard to read |
| Alt+B · Alt+I · Alt+S · Alt+C | bold · italic · strikethrough · code |
| Alt+A | in the reply box: also send to the channel |
| Ctrl+W (Ctrl+Backspace, Alt+Backspace) · Ctrl+U | delete the word before the cursor · delete to the line start |
| Ctrl+A / Home · Ctrl+E / End · Ctrl+← / Ctrl+→ | line start · line end · a word left / right |
| `Ctrl+Space` | the leader while you type: Ctrl+Space then `h` and so on, as `␣h` elsewhere |
| `Ctrl+Space x` · `Ctrl+Space Backspace` | clear the box for good: the text, attachments and mentions (an edit in progress is cancelled, as Esc does) |
| `/` (first in the box) · `@` `#` `:` | a command ([below](#commands)) · mention, channel, emoji autocomplete |

In the suggestions: ↑ (or Shift+Tab) / ↓ move, Tab or Enter inserts the suggestion, Esc closes them.

## Commands

Typed first in the box. ttyak suggests its own and Slack's commands, with what they take; a command typed out in full
runs on Enter. The suggestions also offer the fixed choices (`/dnd 30m` … `off`, `/dnd schedule` kinds and hours,
`/status clear` and `… for today`, `/section rename|delete`, `/workspace` names, `/snippet` languages).

| Command | |
|---|---|
| `/status [:emoji:] [text] [for 30m\|2h\|1d\|today]` · `/status clear` | set or clear your status |
| `/away` · `/active` | appear away · active |
| `/dnd` · `/dnd 30m` (`2h`, `1d`) · `/dnd off` | plain `/dnd` pauses notifications for 30 minutes, or resumes them when paused · pause for a time · resume (ends a pause and the current quiet hours of your notification schedule; the schedule stays; see [Do-not-disturb](Notifications-and-attention#do-not-disturb)) |
| `/dnd schedule` · `/dnd schedule off` · `… every day 9:00-17:00` · `… weekdays 9:00-17:00` | say the notification schedule · turn it off · receive notifications every day / Monday to Friday in these hours (the account's time zone), paused outside them; per-day hours in F6 → Availability |
| `/upload <path> [text]` | upload a file (the path completes as you type: Tab goes into a folder; `~` is your home; names with spaces are quoted for you) |
| `/snippet [lang] [title]` + lines | post the lines after the command (Alt+Enter) as a text snippet |
| `/join [#channel]` · `/leave` | join (default: the channel you are previewing — Ctrl+K finds channels you are not in; a previewed one is fetched again every 15 s, F6 → Messages & media → Refresh previewed channels) · leave the current channel |
| `/create <name>` · `/create-private <name>` | create a channel |
| `/dm @a [@b …]` | open a DM, or a group DM with up to 8 other people |
| `/star` | star or unstar this conversation |
| `/section rename <name>` · `/section delete` | rename or delete this conversation's section |
| `/workspace [name]` | list the profile's workspaces, or switch to one |
| `/prefs` | preferences (also F6) |
| any other `/command` | run by Slack, as its own clients do (`/remind`, `/giphy`, `/shrug`, apps' commands); an answer only you see is marked "only visible to you"; an unknown one says so; a leading space sends the line as text |

## In dialogs

Each dialog shows its keys at the bottom. Esc closes every one of them (`q` too where nothing is typed).

| Dialog | Keys |
|---|---|
| Jump to (Ctrl+K), search (Ctrl+F) | ↑ / ↓ move (Tab: next) · PgUp / PgDn a page (jump to) · Enter goes there (search: searches, then goes to the result) |
| reaction picker (`␣r`) | ↑ / ↓ · PgUp / PgDn ten · Enter reacts with it |
| conversation details (`␣d`) | ← / → (`h` `l`, Shift+Tab / Tab) tabs · ↑ / ↓ (`k` `j`) rows · Enter: a member's profile, a pin's message |
| move to section (`␣m`) | ↑ / ↓ · Home / End · Enter moves there (New section…: name it) |
| snippet view | ↑ / ↓ lines · PgUp / PgDn (and Space) a page · Home / End · Ctrl+U / Ctrl+D · `y` copies the whole snippet |
| image viewer, video player | [Images and video](Images-and-video#the-viewer) |
| save files (`␣s`) | ↑ / ↓ · Space ticks a file, `a` all / none · Tab edits the folder (Tab there completes it) · Enter saves the ticked files |
| context menu (right-click, `␣m`) | ↑ / ↓ · Enter or Space does it |
| the key list (F1) | moves as the snippet view does · Esc, `q`, F1 or `?` closes it |
| preferences (F6) | ↑ / ↓ settings · ← / → (Shift+Tab / Tab) categories · Enter or Space changes the setting · `+` / `-` raise or lower a level (volumes, gain, threshold) · F6 closes too; on F6 → Keys see [Rebinding keys](Rebinding-keys) |
| a text field (a new section's name, keywords) | Enter done · Esc cancels |
| a yes / no question | `y` confirms, any other key is no |
| the update screen | Enter the first button · `u` update now · `y` copy the command or link · `o` the release notes |

## If you miss a key

Single letters and F-keys moved behind `Space`, and every one of them can be put back on a key of its own: see
[Rebinding keys](Rebinding-keys). `ttyak ctl keys-template` prints every action with its keys.

## If it doesn't work

- [Rebinding keys](Rebinding-keys#when-a-binding-does-not-work)
- [Ctrl+Space does nothing](Troubleshooting#ctrlspace-does-nothing)
- [keys.toml or settings.toml is ignored](Troubleshooting#keystoml-or-settingstoml-is-ignored)
- [Copy or paste doesn't work](Troubleshooting#copy-or-paste-doesnt-work)
- everything else: [Troubleshooting](Troubleshooting)
