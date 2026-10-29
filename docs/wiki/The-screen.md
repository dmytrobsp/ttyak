What you see in ttyak and what each part tells you. The keys that move you around are on [Keys](Keys).

## The layout

From left to right: the sidebar, the messages of the open conversation, and a reply pane for a thread on the right. The
thread opens beside the messages when the part right of the sidebar is at least 100 columns wide; narrower, it takes the
messages' place until you close it or go back to the messages or the composer (which closes it). The composer (the
message box) is under the messages, and the status line is at the bottom. With two or more workspaces in the profile a
[rail](#several-workspaces-the-rail) on the far left has a tile for each.

A thumb on a pane's right border shows where you are when it scrolls.

## The sidebar

From the top:

- the **Threads**, **Activity** and **Huddles** rows, which open those views (below)
- the sections, in the order Slack keeps them: **Starred**, your own sections, **Channels** and **Direct messages**.
  Channels and the conversations of your own sections are in order of name; direct messages have the most recently
  active first

Channels and DMs without a message for 30 days (and not created within them) are left out unless they are starred,
unread, open, previewed or have a huddle going on (F6 → Appearance → Hide conversations quiet for: never, 1, 7, 14,
30, 60 or 90 days). Ctrl+K still finds them, and shows them while they are open.

What you have not read is bold, with a bar in the first column. Muted conversations are dimmed and not counted.
A conversation with a draft kept is marked ✎. Each section's name heads a rule, with a blank row between sections
(none in compact mode, F6 → Appearance → Compact). How to fold sections and move conversations between them is on
[Keys](Keys#sections-in-the-sidebar).

## The messages

The selected message is highlighted across the whole width of its pane, every row of it, like a selection in a list
(so is the selected message of the Threads feed and the selected item of the Activity view).

A conversation opened with unread messages has a red "new" line above the first of them. When they start further back
than the newest page, ttyak loads older pages until the first of them is in (up to five more pages). Scrolling,
picking a message, sending, opening a thread, switching conversation or Esc meanwhile stops it; beyond that,
"— more new messages above —" stands at the top until scrolling up loads them. While you read further up, the bottom
border counts the newer messages below ("↓ 3 newer").

The open conversation and thread (and the Threads feed, by its timer) are marked read only while the terminal window
has focus. What comes while it does not stays unread until you are back. This needs a terminal that reports focus
changes; in one that does not, what is on screen counts as read.

ttyak starts on the messages of the conversation you were last in, and opens the thread again if you quit with one
open.

## The status line

On the left, a pill says which mode you are in: NORMAL, INSERT, or the open dialog (JUMP, SEARCH, PREFS, KEYS, MENU,
REACT, IMAGE, VIDEO, …; THREADS, ACTIVITY or HUDDLES in those views). The modes are explained on [Keys](Keys).

Then the unread and mention counts, your status, and a dot for your presence as in Slack: green `● active`, dim
`○ away`. While the connection is down "connecting…" follows it (a dim `…` when the line is at its narrowest). A note,
or a ringing huddle, takes their place for a while.

On the right: the keys of a sequence under way (`␣h`, or `^␣h` for Ctrl+Space), and the hints of where you are,
ending in `? help` (`↑/↓ move · Enter thread · i type · ␣ actions · ? help`). When a newer ttyak is out, a pill with
`⬆` and its version stands at the right end (a click opens the update screen: see [Updates](Updates)).

## Several workspaces: the rail

With two or more workspaces in the profile, a rail on the far left has a tile for each: its logo where it has one and
the terminal draws pictures, else its initials and number in a colour of its own. The shown one is marked by an accent
bar, the others with unread messages by a thinner, dimmer one, and below such a tile `@3` says three unread DMs or
mentions there. A workspace whose session has ended is dim, with `⊘`. A click on a tile switches to it, like
Alt+1 … 9. How workspaces get into a profile is on
[Profiles and sessions](Profiles-and-sessions#several-workspaces-in-one-profile).

## Statuses

A person's status emoji stands right after their name:

- in the sidebar's direct messages (a long name is shortened to keep it; group DMs show none, neither on their rows
  nor in a hover note; a custom emoji, or any emoji while emoji are shown as names, is a dim ◆)
- on their messages (a custom emoji as its picture where the terminal draws pictures)

A DM's title adds the text ("Name — 🌴 Vacationing"), and hovering their DM in the sidebar shows it in the status
line. Someone in a huddle without a status emoji of their own shows 🎧 ("In a huddle"; not on a sidebar row that shows
the huddle already). An expired status is gone at once, without waiting for Slack.

How statuses stay current: ttyak connects to Slack's own websocket gateway, as Slack's web client does, and asks it for
the profile changes of the people it shows. When someone's status or huddle state changes, Slack says so within a
second or two and ttyak looks that one person up.

As a safety net, the people shown are also looked up again every ten minutes, and once after the connection was lost
(when it was down for 30 s or more, or the last such look-up was five minutes ago or more, so a flapping link does not
bring the bursts back). That is up to 50 people at a time: first the authors in the open conversation and thread, then
the people of the DMs and group DMs on screen in the sidebar, then the sidebar's other DMs and group DMs in turn, so
that everyone's comes round. F6 → Messages & media → Refresh people's statuses sets how often
(`[messages] status_refresh_secs`, default 600, 0 = off).

The safety net matters most on the older `rtm.connect` connection, which gets no such pushes. ttyak falls back to it
when Slack offers no gateway, or when the gateway fails twice in a row (refused, or closed before Slack's hello; a
network drop does not count); the gateway is tried again ten minutes later.

## The Threads view

Ctrl+T, `␣ft`, or the **Threads** row atop the sidebar opens it in the main pane, in place of the conversation: a feed
of every thread you are in, unread ones first, so you can read each one where it is. The row turns bold, with a bar
and the number of unread replies, when threads you follow get new ones.

- **A thread's block**: a heading in bold (where — `#channel`, the person or the people of a DM — and who is in it,
  "and 2 others", the day when it is not today; unread ones with a bar and "2 new"); its parent in full as in a
  conversation (a very long one cut after 12 lines with "… more"); "N more replies" when some are hidden; and its last
  3 replies in full — all the unread ones when there are more, with the red "new" line above the first of them.
  Slack's list carries only some replies: for a block on screen that shows fewer than it has, ttyak fetches its newest
  ones once. A parent ttyak cannot read says "message not available". A dim rule and a blank row set each block apart.
- **Up to date**: a dim "You're up to date" line under the threads that had new replies when the view loaded says the
  rest had nothing new (no line when all or none had). It stays until the view loads again.
- **Live changes**: a reply that comes meanwhile goes into its block (one of yours posted from another client too, as
  read), and an edit, a deletion or a reaction shows in its block as it happens. As in Slack, the threads keep their
  place: new threads and the order come when the view loads again.
- **Moving**: ↑/↓ (and a click) select message by message, highlighted across the row. Enter opens the whole thread
  at the selected message, as does a click on a block's header or its "more replies".
- **Replying**: `i` replies in the selected message's thread in the box under the feed ("Reply in #channel thread").
  Its draft is the thread's own; the conversation's comes back when you leave the view. The reply shows in its block.
  However the box gets the focus there, it replies in the selected message's thread (Esc with nothing typed lets go of
  it); a thread open in the thread pane beside the feed is replied to in that pane's box.
- **Message keys**: the keys of a message (`␣r` react, `␣e` `␣d` edit and delete your own, `␣y` `␣Y` `␣o` `␣s` `␣p`
  `␣u` `␣U` `␣hT`) and the right-click menu (`␣m`) work on the selected message — a parent or a reply — as in a
  conversation, and the view stays. Mark unread is offered on a parent, not on a reply. A reaction, an edit or a
  deletion shows in its block at once; nothing opens beside the feed and nothing is marked read for it.
- **Editing**: an edit goes into the box under the feed ("Editing message"; Enter saves, Esc drops it; `i` goes back
  to it — on another thread too, which the status line says: one edit stays open until you save or cancel it). Leaving
  the view or quitting keeps it as its thread's draft (or in its box, when that is the thread's own and empty) unless
  that thread has a draft already, and the status line says which; then quitting asks for Ctrl+Q a second time. Once
  the edit is over the box has again what it held — a reply half typed stays, as does any thread's draft.
- **Read marks**: a thread is marked read on Slack once its whole block — only for one taller than the view, its unread
  replies — has been on screen for 2 seconds, not while you scroll past (F6 → Messages & media → Mark feed threads
  read, `[threads] read_after_secs`, 0 = only when opened). Its bar and count go; its "new" line stays until the view
  loads again. Opening it or replying marks it read too.
- `r` or Ctrl+T again loads it again; more load as the end of the feed comes on screen; Esc (or `q`) goes back to the
  conversation you were in.

## The Activity view

Ctrl+A, `␣fa`, or the **Activity** row in the sidebar (under Threads) opens it in the main pane. It lists mentions,
reactions to your messages, thread replies, new direct messages, and the rest of what Slack's Activity badge counts —
new messages in a channel you get notified of, an app's messages, being added to a channel, a mention in a channel you
are not in, someone new to welcome, Slack Connect invitations and list items — newest first, under date headers
("Today", "Yesterday", then the day as F6 → Language & region writes it).

- **An item** shows who (in a thread, "+N" others who replied in it), their status emoji and when; the message itself
  as a conversation draws it (links, code, custom emoji; up to three lines, then "…"; "message not available" when it
  is gone); and where it happened — "Mention in #general thread", "Thread in #general · 2 new replies", "Direct
  message", "Reacted (🎉 1) to:" over your message, "New messages in #general · 3 new", "App message in …", "You were
  added to #general", "Mention in #general, a channel you are not in" — with the unread bar on unread ones.
- **Tabs** All · DMs · Mentions · Threads in a bar on top: `]` / `[` next / previous, `1`–`4` straight to one, or click
  it. Each tab keeps its list while the view is open, and loads again when shown.
- **`u`** keeps only the unread items (again: all). It stays on until you turn it off — across tabs and each time the
  view opens — until ttyak quits.
- **More**: older items load as the end of the list comes on screen. With only the unread ones kept, loading goes on
  until one shows or the list ends, at most 5 pages by themselves: then "There is more: move down to load it" at the
  end, and moving down, `u`, another tab, `r` or opening the view again goes on. A page that could not be loaded, or
  one with nothing new, is said there and asked for again the same way.
- **Enter goes there**: a DM opens the conversation; a mention or a reaction the message (a reply in its thread); a
  thread item the thread at its first unread reply; a channel item the channel at its newest message; an app's item
  its DM; an "added to" item the channel (one you are not in is previewed). A Slack Connect invitation or a list item
  has nowhere to go in ttyak, so Enter marks it read and the status line says to open it in Slack. With nothing shown,
  Enter does nothing. Enter on an unread item marks it read on Slack as Slack's own client does.
- `r` or Ctrl+A again loads the tab again; Esc (or `q`) goes back to the conversation you were in. New items show up
  in the open view as they come.
- **Message keys**: on an item that is a message (a mention, a reply, a DM, your message someone reacted to) the keys
  of a message (`␣r` react, `␣e` `␣d` edit and delete your own, `␣y` `␣Y` `␣o` `␣s` `␣p` `␣u` `␣U` `␣hT`) and its
  right-click menu (`␣m`) work as in a conversation, and the view stays. A reaction, an edit or a deletion shows on
  the item at once (as do other people's, live). Mark unread marks the message's own conversation (not a reply's,
  which is in a thread). On an item that is no message they say so (or that its message is still loading).
- **Editing**: an edit opens a box under the view, only while it lasts (Enter saves, Esc drops it, `i` goes back to
  it). The conversation's draft is kept aside meanwhile and comes back. Leaving the view or quitting with it open keeps
  it as the draft of the message's thread — or conversation, for a top-level message. When that box is on screen and
  empty, the edit simply stays in it, "Editing message" — unless a draft is kept there already, and the status line
  says which; then quitting asks for Ctrl+Q a second time, and another workspace waits until the edit is saved or
  cancelled.

The **Activity** row turns bold, with a bar and a number, while there is unread activity: Slack's own Activity badge
(unread mentions, thread replies, direct messages and the other kinds above; reactions never count, as in Slack). Every
kind the badge counts is listed in the All tab, so its unread items and the number agree. The number is kept current
from Slack's live events: a new mention, DM or thread reply raises it, reading the item here or on another device
lowers it, with no polling (asked for again only after a reconnect). A workspace whose counts have no such badge shows
the All tab's unread items, counted when the workspace starts and each time it loads.

## The Huddles view

`␣fh`, or the **Huddles** row in the sidebar (under Activity), opens it in the main pane: every huddle going on now and
who is in it, then the recent ones. Huddles themselves are on [Huddles](Huddles).

- **Live now** lists the huddles in conversations you are a member of (not muted ones, as Slack does), yours first,
  each marked with a green "● Live": where (`#channel`, the person or the people of a DM), how long it has been going
  ("40 s", "12 min", "1 h", "1 h 5 min"), and who is in it with their status emoji. A huddle in a conversation ttyak
  doesn't know is named by asking Slack once ("#name"), or "a conversation you are not in" when Slack won't say.
- **Recent** lists the ended huddles you were in or missed (from Slack's huddle history, newest first, more loading as
  the end comes on screen): where, "3 h ago · 25 min", who took part, "N replies" when its thread has some, and a
  missed one as "Missed huddle" in the alert colour.
- ↑/↓ (and a click) select. Enter on a live one opens its conversation, on a recent one its thread; the huddle key
  (`␣hh`) on a live one joins it. `r` or `␣fh` again loads it again; Esc (or `q`) goes back to the conversation you
  were in.

A badge on the **Huddles** row counts the live huddles you can join (not the one you are in), kept current from Slack's
live events and asked for again at start and after each reconnect.

Selecting a conversation in the sidebar (or jumping to one) leaves a view. ttyak always starts on a conversation, never
a view.

## If it doesn't work

- [The thread doesn't open beside the conversation](Troubleshooting#the-thread-doesnt-open-beside-the-conversation)
- [Emoji or symbols show as boxes](Troubleshooting#emoji-or-symbols-show-as-boxes)
- [Colours look wrong](Troubleshooting#colours-look-wrong)
- [A workspace is signed out](Troubleshooting#a-workspace-is-signed-out)
- everything else: [Troubleshooting](Troubleshooting)
