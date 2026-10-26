Some things Slack's own app does that ttyak doesn't do yet. None of them is a bug: ttyak just has no way to do them
for now. Use Slack's app or the web page for these. The list is tracked in one
[issue on GitHub](https://github.com/dmytrobsp/ttyak/issues/43) — say there which one you miss.

## The list

- **Workspaces keep nothing when you switch.** Switching loads the other workspace afresh. The workspaces you are not
  in are checked every 30 seconds for unread counts and notifications, but their conversations aren't kept.
- **Buttons in messages.** Buttons and menus in a message, such as an app's answer to a command, show as text. You
  can't click them.
- **Animated pictures.** A GIF shows its first frame only.
- **Slack's other sidebar sections.** The sidebar shows Starred, your own sections, Channels and Direct messages.
  Slack Connect, Recent apps and Slack's other built-in sections don't show, and a section can't have its own sort.
- **Managing channels.** ttyak can't archive or rename a channel, invite people to it or change its topic.
- **Editing snippets.** You can read a snippet (a code or text file in a message) but not edit it.

## If it doesn't work

- [Reporting a bug](Troubleshooting#reporting-a-bug)
- everything else: [Troubleshooting](Troubleshooting)
