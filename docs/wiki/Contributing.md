How to work on ttyak is in [CONTRIBUTING.md](https://github.com/dmytrobsp/ttyak/blob/master/CONTRIBUTING.md) in the
repository. It covers:

- building and testing: `cargo build`, `cargo test`, a clean build in a container
  (`tools/container-build.sh debian|ubuntu|fedora|arch`), and cross-building the Windows binary from Linux (see also
  [Building from source](Building-from-source))
- running against Slack: a workspace of your own for testing, a profile per account (`--profile`), and `ctl` to let a
  second account play the other person (see [Scripting](Scripting))
- the logs, and the variables debug builds read for tests (`TTYAK_FAIL`, `TTYAK_DAYS_LATER`, `TTYAK_HUDDLE_DROP`)
- measuring memory with the heap profiler
- a map of the code (also on [Project layout](Project-layout)) and the style: match the surrounding code, and do not
  reformat files you are not otherwise changing (the code is not formatted with `rustfmt`)

Bugs and ideas go to the [issues](https://github.com/dmytrobsp/ttyak/issues) (a form for each); leave out anything
private, such as tokens, cookies, message text and names.

## This wiki

The wiki's pages are edited in [`docs/wiki/`](https://github.com/dmytrobsp/ttyak/tree/master/docs/wiki) in the main
repository, like the code, and published here at each release: the wiki shows the pages of the latest release, so a
change on master appears here with the next one.
