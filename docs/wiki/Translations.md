The interface can be translated, one string at a time, with a locale file of your own.

## Making one

`ttyak ctl locale-template` prints a locale file with every translatable string, each as `"English text" = ""`. Save
it as `locales/<code>.toml` next to `settings.toml`, for German on Linux:

```sh
mkdir -p ~/.config/ttyak/locales
ttyak ctl locale-template > ~/.config/ttyak/locales/de.toml
```

(macOS: `~/Library/Application Support/ttyak/locales/`, Windows: `%APPDATA%\ttyak\config\locales\`.) Fill in the
right-hand sides; an empty one stays English. Keep `{n}`, `{key}` and the like as they are: ttyak puts the value there.

Then in F6 → Language & region → Language press Enter, type `de` and press Enter: the file is read at once, and a file
that is missing or does not parse is named with the reason. `en` (or empty) is the built-in English.

## What is translated

So far: the key names and labels (F1, which-key, F6 → Keys), the preferences and their help, the commands list, overlay
titles and the status-bar hints, some status-line notes (links, huddles), the Threads, Activity and Huddles views' own
words, the update notice and the update screen, and the lines of `ttyak ctl check` and `ttyak ctl status`. Messages,
most status-line messages and the command line's own help stay in English.

## If it doesn't work

- [Troubleshooting](Troubleshooting)
