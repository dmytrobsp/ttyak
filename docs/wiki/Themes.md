ttyak comes with three colour themes — `dark` (the default), `light` and `terminal` (your terminal's own colours) — and
takes your own.

## Choosing one

F6 → Appearance → Colour theme opens a picker of the built-in themes and your own, each with a few of its colours:
type to filter, ↑/↓ show each one on the whole screen at once, Enter keeps it, Esc goes back to the one before. A theme
file that does not load says why ("can't load" beside its name).

To tell workspaces apart at a glance, F6 → Appearance → Theme for this workspace gives the workspace you are in a theme
of its own (the same picker, with "(colour theme)" for none). `settings.toml` keeps it under
`[appearance.workspace_themes]` by workspace id.

On a terminal without 24-bit colour, use the `terminal` theme (see [Requirements](Requirements#the-terminal)).

## Your own theme

A theme is a TOML file in `themes/` next to `settings.toml` (on Linux `~/.config/ttyak/themes/`); its name is the
file's name (a file named `dark`, `light` or `terminal` is left out: those stay the built-in ones). It has
`base = "dark"` (or `light`, `terminal`) plus any colours to change, as `"#ff8800"`, an ANSI name such as
`"light-red"`, or a number 0–255. Start from a complete one, edit it, and install it (checked first):

```sh
ttyak ctl theme-template dark > sunset.toml
ttyak ctl theme-import sunset.toml
```

`theme-import` copies the file into `themes/` once it loads (an unknown entry or a bad colour is refused, with the
reason); then choose it in F6 → Appearance. Share themes by passing the file around.

The entries: `accent` (focused borders, titles, key names), `selection` (the selected row), `hover` (the row under the
mouse), `dim` (times, hints, separators), `strong` (unread conversations), `read` (read ones), `good` (online dots,
values in F6), `danger` (errors, delete questions), `own` (the bar along your own messages; huddles), `mention`, `link`
(web links), `app_link` (links into your own Slack), `code_fg`, `code_bg`, `quote_bar`, `badge_fg` and `badge_bg`
(mention counts), `pin`, `call_bg` (the call bar), `authors` (a list of ten colours for author names) and `syntax` (the
syntect theme for snippets).

## If it doesn't work

- [Troubleshooting](Troubleshooting)
