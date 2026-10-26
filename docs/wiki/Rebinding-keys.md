Every key in ttyak can be changed: in the app (F6 → Keys) or by hand in `keys.toml`. The default keys are on
[Keys](Keys).

## In the app: F6 → Keys

F6 → Keys lists every action under its group, with the which-key settings above them (show it, its corner, the delay
before it opens, its largest height, icons) and the two leaders.

- `/` filters the list (Enter keeps the filter, Esc clears it).
- Enter on a row records the new keys: press them, or a whole sequence such as `Space r`. The recording ends after a
  1 s pause, and any key counts.
- Delete (or Backspace) unbinds the action, `r` restores its default.
- Enter on a group renames it, `i` changes its icon (empty: the default).

Each row's help says where the action works, its keys, its default when you changed it, and its name in
`keys.toml`.

Only the one entry you change is written to `keys.toml`; your comments and the order of the file stay. A `keys.toml`
that is a symbolic link (dotfiles) stays a link: its target is written.

## keys.toml

| System | File |
|---|---|
| Linux | `~/.config/ttyak/keys.toml` (or under `$XDG_CONFIG_HOME`) |
| macOS | `~/Library/Application Support/ttyak/keys.toml` |
| Windows | `%APPDATA%\ttyak\config\keys.toml` |

`ttyak ctl keys-template` prints a complete `keys.toml` with every default and a comment for each action — a good start
to edit by hand. Each table is a context: `[global]`, `[sidebar]`, `[messages]`, `[huddle_view]`, `[composer]`,
`[completion]`, and one per dialog or view (`[switcher]` for Ctrl+K, `[search]`, `[reaction_picker]`, `[details]`,
`[profile]`, `[activity]`, `[threads]`, `[huddles]`, `[move_to_section]`, `[text_field]`, `[snippet]`,
`[image_viewer]`, `[video_player]`, `[confirm]`, `[help]`, `[preferences]`, `[menu]`, `[save]`, `[update]`). Each
entry replaces an action's keys, and a line left out keeps its default:

```toml
leader = "space"              # the leader in NORMAL; "" for none
insert_leader = "ctrl+space"  # the leader while typing and in dialogs

[groups]                      # names and icons in the which-key panel
"<leader> h" = { name = "calls", icon = "🎧" }

[labels.messages]             # a label and icon for one action
react = { label = "quick", icon = "⚡" }

[messages]
react = ["<leader> r", "R"]   # several keys or sequences for one action
pin = []                      # unbound
copy = "y"                    # back on a key of its own
```

ttyak reads `keys.toml` when it starts and when it switches workspace; F6 → Keys writes it for you.

## Writing keys

- A sequence is keys separated by spaces: `"<leader> r"` or `"ctrl+x ctrl+s"`. `<leader>` is Space in the panes and
  views, and the insert leader (Ctrl+Space) while you type and in dialogs.
- A key is written like `ctrl+k`, `alt+up`, `shift+tab`, `G`, `f5`, `space`, `pageup`, `enter`, `esc`, `backspace`,
  `delete`, `plus`.
- An action can have a key of its own and sequences at once.
- Plain characters cannot be bound where you type: the composer, Ctrl+K, search, the reaction picker and text fields.
  There use `insert_leader` sequences. A plain character bound in `[global]` acts only in the panes and views
  (NORMAL), never while you type.
- `insert_leader` must be a key that types nothing (with Ctrl or Alt, or an F-key). With `insert_leader = ""` and a
  leader that types text, typing and dialogs have no leader at all.

Existing `keys.toml` files keep working: your binding beats the default. A default that clashes with one of your
bindings (the same keys, or one that starts the other) gives way to yours.

## When a binding does not work

Problems are shown in the status bar at startup: the first one, with "(and N more key problems, see the log)" when
there are several; all of them go to the log. The default stays in place for any entry that has a problem. A key
your terminal or system takes for itself never reaches ttyak: Ctrl+Space on macOS (input sources) and with some
Windows input methods is the usual one — rebind `insert_leader` in F6 → Keys.

## If it doesn't work

- [Troubleshooting](Troubleshooting)
