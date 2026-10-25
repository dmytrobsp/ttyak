//! `ttyak ctl check`: what the terminal answered and what the system has, and one sample line per kind of glyph for
//! the user to judge (a program cannot tell which font the terminal draws with). No Slack, no network.

use crate::tui::media::decrqm as mode;

/// What was found. A terminal row is yes, no, or None (the terminal did not say); a system row is its value, or None
/// (could not be told).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Findings {
    pub terminal: Vec<(String, Option<bool>)>,
    pub system: Vec<(String, Option<String>)>,
    /// The console does not take escape sequences (Windows' legacy console with no virtual terminal processing).
    pub no_escapes: bool,
}

/// One sample line per kind of glyph, for the user to look at: (the group's name, its glyphs).
pub const GLYPHS: &[(&str, &str)] = &[
    ("symbols", "● ○ ◆ ★ ⏎ ↪ ⬆ ✕"),
    ("arrows", "← → ↑ ↓ ↔ ↩"),
    ("boxes and blocks", "─│┌┘▀▄█░"),
    ("emoji", "👀 🎧 📎 🎉"),
    ("CJK", "日本語 中文 한국어"),
];

// Terminal rows.
const UTF8: &str = "UTF-8 text";
const TRUE_COLOUR: &str = "true colour (the built-in themes; else use the terminal theme)";
const MOUSE: &str = "mouse reporting";
const FOCUS: &str = "focus reporting (bell, badge, read marks)";
const PASTE: &str = "bracketed paste";
const LINKS: &str = "links (OSC 8; see the sample below)";
const OSC99: &str = "notifications by the terminal (OSC 99)";
const PICTURES: &str = "pictures ({modes})";
const NO_PICTURES: &str = "half blocks only";
const SIZE: &str = "at least 80 × 24 cells (now {cols} × {rows})";
// System rows.
const OPENER: &str = "link opener";
const NOTIFIER: &str = "notification service";
const CAPABILITIES: &str = "notification service offers";
const DND: &str = "do not disturb";
const SHOWN_BY: &str = "notifications shown by";
const SOUND_BY: &str = "notification sound";
const CLICK: &str = "a click on a notification";
const RAISE: &str = "bringing the window forward";
const FFMPEG: &str = "ffmpeg and ffprobe (video, cameras, screen sharing)";
const GSTREAMER: &str = "GStreamer (screen sharing on Wayland)";
const CLIPBOARD: &str = "clipboard";
// Values.
const FOUND: &str = "found";
const MISSING: &str = "not found";
const BUILT_IN: &str = "built in";
const YES: &str = "yes";
const NO: &str = "no";
const NOT_KNOWN: &str = "not known";
const MAC_SERVICE: &str = "macOS Notification Center";
const WINDOWS_SERVICE: &str = "Windows toasts";
const NO_SERVER: &str = "none answered";
const WAYLAND_CLIPBOARD: &str = "Wayland (needs a compositor with data-control; else the terminal's OSC 52)";
const SYSTEM_CLIPBOARD: &str = "the system's";
// Report.
const TITLE: &str = "ttyak ctl check — what this terminal and system offer";
const TERMINAL: &str = "The terminal";
const SYSTEM: &str = "The system";
const NO_ESCAPES: &str = "This console does not take escape sequences (its virtual terminal processing could not be turned on), so it was not asked: run ttyak in Windows Terminal.";
const NOT_A_TERMINAL: &str = "Not a terminal (stdin or stdout is not one: piped or redirected): the terminal part is skipped. Run it in your terminal.";
const GLYPHS_HEAD: &str = "Glyphs: the | at the end of each line should sit in one column";
const LOOK_FOR: &str = "Look for empty boxes or misaligned columns";
const FONTS_HINT: &str = "Boxes: install the fonts listed under Requirements in the README, or turn on F6 → Messages & media → Show emoji as :names:. Misaligned rows: the terminal measures emoji differently.";
const LINKS_NAME: &str = "links (OSC 8)";
const LINK_TEXT: &str = "a link to ttyak.sh";

pub const WORDS: &[&str] = &[
    UTF8, TRUE_COLOUR, MOUSE, FOCUS, PASTE, LINKS, OSC99, PICTURES, NO_PICTURES, SIZE, OPENER, NOTIFIER, CAPABILITIES, DND, SHOWN_BY, SOUND_BY, CLICK, RAISE, FFMPEG, GSTREAMER,
    CLIPBOARD, FOUND, MISSING, BUILT_IN, YES, NO, NOT_KNOWN, MAC_SERVICE, WINDOWS_SERVICE, NO_SERVER, WAYLAND_CLIPBOARD, SYSTEM_CLIPBOARD, TITLE, TERMINAL, SYSTEM, NO_ESCAPES, NOT_A_TERMINAL,
    GLYPHS_HEAD, LOOK_FOR, FONTS_HINT, LINKS_NAME, LINK_TEXT, "symbols", "arrows", "boxes and blocks", "emoji", "CJK",
];

/// Whether `reply` ends with the terminal's answer to DA1 (`CSI ? … c`), which ends the answers it gives: without it the
/// terminal answered nothing, so nothing it did not answer is known.
fn answered(reply: &[u8]) -> bool {
    crate::tui::media::da1_ended(reply)
}

/// The pictures and OSC 99 rows from what the picture query found (`graphics`: a protocol, or half blocks only; not
/// what `TTYAK_GRAPHICS` forces), whether
/// OSC 99 was offered, and the answers fenced by DA1 (`reply`): a protocol the terminal answered stands without DA1; half
/// blocks only and OSC 99 are known only when the terminal was heard.
fn heard(graphics: Option<bool>, osc99: bool, reply: &[u8]) -> (Option<bool>, Option<bool>) {
    let heard = answered(reply);
    (graphics.filter(|g| *g || heard), heard.then_some(osc99))
}

/// 24-bit colour from `$COLORTERM`, or Windows Terminal (`$WT_SESSION`; it has 24-bit colour but sets no `COLORTERM`)
/// unless inside a `multiplexer` (tmux or screen: `$TMUX`, `$STY`), which inherits `WT_SESSION` but draws for itself: a
/// terminal that does not say may still have it.
fn true_colour(colorterm: Option<&str>, wt_session: Option<&str>, multiplexer: bool) -> Option<bool> {
    let said = colorterm.is_some_and(|c| matches!(c.trim(), "truecolor" | "24bit"));
    (said || (!multiplexer && wt_session.is_some_and(|s| !s.is_empty()))).then_some(true)
}

/// Whether ttyak runs inside tmux (`$TMUX`) or screen (`$STY`); set but empty says nothing (as [`osc99::in_tmux`]).
///
/// [`osc99::in_tmux`]: crate::tui::osc99::in_tmux
fn multiplexer(tmux: Option<&str>, sty: Option<&str>) -> bool {
    [tmux, sty].into_iter().flatten().any(|v| !v.is_empty())
}

/// UTF-8: yes on Windows (`windows`; a console takes ttyak's text as UTF-16 whatever its code page, and a pty such as
/// mintty's or msys's gets UTF-8, mintty's default — the locale variables say nothing there); elsewhere from the locale
/// variables (`$LC_ALL`, `$LC_CTYPE`, `$LANG`, the first one set decides); None when none is set.
fn utf8(vars: &[Option<&str>], windows: bool) -> Option<bool> {
    if windows {
        return Some(true);
    }
    let v = vars.iter().flatten().find(|v| !v.is_empty())?.to_lowercase();
    Some(v.contains("utf-8") || v.contains("utf8"))
}

/// The console's escape sequences on for as long as it lives (Windows: virtual terminal processing, the console's mode
/// put back when it drops; other terminals take them anyway). `escapes` is false when they could not be turned on.
struct Vt {
    #[cfg(windows)]
    restore: Option<(windows_sys::Win32::Foundation::HANDLE, u32)>,
    escapes: bool,
}

impl Vt {
    fn on() -> Vt {
        #[cfg(windows)]
        {
            use windows_sys::Win32::System::Console::{ENABLE_VIRTUAL_TERMINAL_PROCESSING, GetConsoleMode, GetStdHandle, STD_OUTPUT_HANDLE, SetConsoleMode};
            // SAFETY: plain console calls on this process's own standard output.
            let out = unsafe { GetStdHandle(STD_OUTPUT_HANDLE) };
            let mut mode = 0;
            // Not a console (piped: the terminal part is skipped), or on already.
            if unsafe { GetConsoleMode(out, &mut mode) } == 0 || mode & ENABLE_VIRTUAL_TERMINAL_PROCESSING != 0 {
                return Vt { restore: None, escapes: true };
            }
            if unsafe { SetConsoleMode(out, mode | ENABLE_VIRTUAL_TERMINAL_PROCESSING) } == 0 {
                return Vt { restore: None, escapes: false };
            }
            Vt { restore: Some((out, mode)), escapes: true }
        }
        #[cfg(not(windows))]
        Vt { escapes: true }
    }
}

impl Drop for Vt {
    fn drop(&mut self) {
        #[cfg(windows)]
        if let Some((out, mode)) = self.restore {
            // SAFETY: the handle and mode read in `on`.
            unsafe { windows_sys::Win32::System::Console::SetConsoleMode(out, mode) };
        }
    }
}

/// Raw mode for as long as it lives, off again when it drops (also on a panic).
struct Raw;

impl Raw {
    fn on() -> Option<Raw> {
        crossterm::terminal::enable_raw_mode().ok().map(|()| Raw)
    }
}

impl Drop for Raw {
    fn drop(&mut self) {
        let _ = crossterm::terminal::disable_raw_mode();
    }
}

/// Whether `name` is an executable on PATH.
fn on_path(name: &str) -> bool {
    let Some(path) = std::env::var_os("PATH") else { return false };
    let exts: Vec<String> = if cfg!(windows) { std::env::var("PATHEXT").unwrap_or(".EXE".into()).split(';').map(|e| e.to_lowercase()).collect() } else { vec![String::new()] };
    std::env::split_paths(&path).any(|dir| exts.iter().any(|ext| dir.join(format!("{name}{ext}")).is_file()))
}

fn found(name: &str) -> String {
    use crate::tui::i18n::tr;
    tr(if on_path(name) { FOUND } else { MISSING }).into_owned()
}

/// What the terminal answers: queried in raw mode for a moment, before anything else reads the input (not asked when
/// the console takes no `escapes`: the questions would show as text).
fn terminal(escapes: bool) -> (Vec<(String, Option<bool>)>, Option<bool>) {
    use crate::tui::i18n::tr;
    use std::io::IsTerminal;
    if !std::io::stdin().is_terminal() || !std::io::stdout().is_terminal() {
        return (Vec::new(), None);
    }
    let var = |n: &str| std::env::var(n).ok();
    let mut rows = vec![(tr(UTF8).into_owned(), utf8(&[var("LC_ALL").as_deref(), var("LC_CTYPE").as_deref(), var("LANG").as_deref()], cfg!(windows)))];
    rows.push((tr(TRUE_COLOUR).into_owned(), true_colour(var("COLORTERM").as_deref(), var("WT_SESSION").as_deref(), multiplexer(var("TMUX").as_deref(), var("STY").as_deref()))));
    let (mut graphics, mut osc99, mut modes) = (None, false, Vec::new());
    if let Some(_raw) = escapes.then(Raw::on).flatten() {
        let media = crate::tui::media::Media::detect("auto");
        (graphics, osc99) = (Some(media.detected_graphics()), media.osc99);
        modes = crate::tui::media::device_attributes("\x1b[?1000$p\x1b[?1004$p\x1b[?2004$p");
    }
    rows.push((tr(MOUSE).into_owned(), mode(&modes, 1000)));
    rows.push((tr(FOCUS).into_owned(), mode(&modes, 1004)));
    rows.push((tr(PASTE).into_owned(), mode(&modes, 2004)));
    rows.push((tr(LINKS).into_owned(), None));
    let (graphics, osc99) = heard(graphics, osc99, &modes);
    rows.push((tr(OSC99).into_owned(), osc99));
    let pictures = match graphics {
        Some(true) => crate::tui::media::drawable().iter().filter(|m| **m != "auto" && **m != "halfblocks").copied().collect::<Vec<_>>().join(", "),
        Some(false) => tr(NO_PICTURES).into_owned(),
        None => tr(NOT_KNOWN).into_owned(),
    };
    rows.push((tr(PICTURES).replace("{modes}", &pictures), graphics));
    let (cols, rows_n) = crossterm::terminal::size().unwrap_or((0, 0));
    rows.push((tr(SIZE).replace("{cols}", &cols.to_string()).replace("{rows}", &rows_n.to_string()), (cols > 0).then_some(cols >= 80 && rows_n >= 24)));
    (rows, osc99)
}

/// The notification service and how bubbles, their sound and clicks go (asked of the same code that shows them), in a
/// runtime of its own.
fn notifications(terminal: Option<String>) -> Vec<(String, Option<String>)> {
    use crate::tui::{i18n::tr, notify::Notifier, system_dnd};
    let work = move || -> Vec<(String, Option<String>)> {
        let Ok(rt) = tokio::runtime::Builder::new_current_thread().enable_all().build() else { return Vec::new() };
        rt.block_on(async {
            let mut rows = Vec::new();
            let sound = crate::tui::settings::load().0.notifications.sound;
            match system_dnd::discover().await {
                Ok(env) => {
                    let service = if cfg!(target_os = "macos") {
                        tr(MAC_SERVICE).into_owned()
                    } else if cfg!(windows) {
                        tr(WINDOWS_SERVICE).into_owned()
                    } else {
                        system_dnd::name(&env.server)
                    };
                    rows.push((tr(NOTIFIER).into_owned(), Some(service)));
                    if cfg!(all(unix, not(target_os = "macos"))) {
                        rows.push((tr(CAPABILITIES).into_owned(), Some(env.capabilities.join(", "))));
                    }
                    let asked = system_dnd::suppressed(&env.server).await;
                    rows.push((tr(DND).into_owned(), Some(system_dnd::describe(&env.server, asked))));
                }
                Err(_) => rows.push((tr(NOTIFIER).into_owned(), Some(tr(NO_SERVER).into_owned()))),
            }
            let notifier = Notifier::new();
            if let Some(name) = terminal {
                notifier.use_terminal(name);
            }
            notifier.route(&sound);
            for _ in 0..20 {
                if notifier.route_known() {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
            let route = notifier.route(&sound);
            rows.push((tr(SHOWN_BY).into_owned(), Some(route.shown_by)));
            rows.push((tr(SOUND_BY).into_owned(), Some(route.sound_by)));
            rows.push((tr(CLICK).into_owned(), Some(route.click)));
            rows
        })
    };
    std::thread::spawn(work).join().unwrap_or_default()
}

/// What the terminal answered and what the system has (`escapes`: the console takes escape sequences). Nothing leaves
/// the machine.
pub fn findings(escapes: bool) -> Findings {
    use crate::tui::{i18n::tr, osc99, raise};
    if let Err(e) = crate::tui::i18n::set(&crate::tui::settings::load().0.region.language) {
        tracing::debug!("language: {e}");
    }
    let (terminal, osc99_answered) = terminal(escapes);
    let mut system = Vec::new();
    let opener = if cfg!(windows) { tr(BUILT_IN).into_owned() } else { found(if cfg!(target_os = "macos") { "open" } else { "xdg-open" }) };
    system.push((tr(OPENER).into_owned(), Some(opener)));
    system.extend(notifications((osc99_answered == Some(true) && osc99::wanted()).then(osc99::terminal_name)));
    system.push((tr(RAISE).into_owned(), Some(raise::describe(&raise::detect(&raise::env())))));
    let ffmpeg = if on_path("ffmpeg") && on_path("ffprobe") { tr(FOUND).into_owned() } else { tr(MISSING).into_owned() };
    system.push((tr(FFMPEG).into_owned(), Some(ffmpeg)));
    if cfg!(all(unix, not(target_os = "macos"))) {
        system.push((tr(GSTREAMER).into_owned(), Some(found("gst-launch-1.0"))));
        let wayland = std::env::var("WAYLAND_DISPLAY").is_ok_and(|v| !v.is_empty());
        system.push((tr(CLIPBOARD).into_owned(), Some(tr(if wayland { WAYLAND_CLIPBOARD } else { SYSTEM_CLIPBOARD }).into_owned())));
    }
    Findings { terminal, system, no_escapes: !escapes }
}

/// The report as text. Every word goes through `tr`.
pub fn report(f: &Findings) -> String {
    use crate::tui::i18n::tr;
    use unicode_width::UnicodeWidthStr;
    let mut out = format!("{}\n", tr(TITLE));
    if f.terminal.is_empty() {
        out += &format!("\n{}\n", tr(NOT_A_TERMINAL));
    } else {
        out += &format!("\n{}\n", tr(TERMINAL));
        for (label, value) in &f.terminal {
            let value = match value {
                Some(true) => tr(YES),
                Some(false) => tr(NO),
                None => tr(NOT_KNOWN),
            };
            out += &format!("  {label}: {value}\n");
        }
        if f.no_escapes {
            out += &format!("  {}\n", tr(NO_ESCAPES));
        }
    }
    out += &format!("\n{}\n", tr(SYSTEM));
    for (label, value) in &f.system {
        let value = value.as_deref().map_or_else(|| tr(NOT_KNOWN).into_owned(), String::from);
        out += &format!("  {label}: {value}\n");
    }
    if !f.terminal.is_empty() {
        out += &format!("\n{}\n", tr(GLYPHS_HEAD));
        for (group, glyphs) in GLYPHS {
            // The bar goes to the same cell on every line when the terminal measures the glyphs as expected.
            let name = tr(group);
            let name_pad = " ".repeat(18usize.saturating_sub(name.width()));
            let pad = 24usize.saturating_sub(glyphs.width());
            out += &format!("  {name}{name_pad}{glyphs}{}|\n", " ".repeat(pad));
        }
        if !f.no_escapes {
            out += &format!("  {}: \x1b]8;;https://ttyak.sh\x1b\\{}\x1b]8;;\x1b\\\n", tr(LINKS_NAME), tr(LINK_TEXT));
        }
        out += &format!("\n{}. {}\n", tr(LOOK_FOR), tr(FONTS_HINT));
    }
    out
}

pub fn run() {
    // (On until the report, its link sample included, is printed.)
    let vt = Vt::on();
    print!("{}", report(&findings(vt.escapes)));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::i18n::tr;

    fn sample() -> Findings {
        Findings {
            terminal: vec![("UTF-8 text".into(), Some(true)), ("mouse reporting".into(), Some(false)), ("focus reporting".into(), None)],
            system: vec![("link opener".into(), Some("found".into())), ("ffmpeg".into(), None)],
            no_escapes: false,
        }
    }

    #[test]
    fn the_report_lists_every_glyph_group_and_says_what_to_look_for() {
        let r = report(&sample());
        for (group, glyphs) in GLYPHS {
            let line = r.lines().find(|l| l.contains(&*tr(group))).unwrap_or_else(|| panic!("no line for {group}: {r}"));
            assert!(line.contains(glyphs), "{line}");
        }
        for g in ["● ○ ◆ ★ ⏎ ↪ ⬆ ✕", "─│┌┘▀▄█░", "👀 🎧 📎 🎉", "日本語 中文 한국어"] {
            assert!(GLYPHS.iter().any(|(_, s)| s.contains(g)), "{g}");
        }
        assert!(r.contains("Look for empty boxes or misaligned columns"), "{r}");
    }

    #[test]
    fn what_is_not_known_says_so() {
        let r = report(&sample());
        assert!(r.contains("focus reporting: not known"), "{r}");
        assert!(r.contains("ffmpeg: not known"), "{r}");
        assert!(r.contains("mouse reporting: no") && r.contains("UTF-8 text: yes") && r.contains("link opener: found"), "{r}");
    }

    #[test]
    fn outside_a_terminal_the_terminal_part_is_skipped() {
        let r = report(&Findings { terminal: vec![], system: vec![("ffmpeg".into(), Some("found".into()))], no_escapes: false });
        assert!(r.contains("Not a terminal"), "{r}");
        assert!(!r.contains("Look for empty boxes"), "{r}");
        assert!(r.contains("ffmpeg: found"), "{r}");
    }

    #[test]
    fn every_word_goes_through_tr_and_is_in_the_locale_template() {
        let template = crate::tui::i18n::template();
        for w in WORDS {
            assert!(template.contains(&format!("\"{w}\" = \"\"")), "{w}");
        }
        assert!(WORDS.contains(&"Look for empty boxes or misaligned columns"));
    }

    #[test]
    fn the_answers_to_the_mode_questions() {
        assert_eq!(mode(b"\x1b[?1004;2$y\x1b[?62;c", 1004), Some(true));
        assert_eq!(mode(b"\x1b[?1004;1$y", 1004), Some(true));
        assert_eq!(mode(b"\x1b[?1004;3$y", 1004), Some(true));
        assert_eq!(mode(b"\x1b[?1004;0$y", 1004), Some(false));
        assert_eq!(mode(b"\x1b[?1004;4$y", 1004), Some(false));
        assert_eq!(mode(b"\x1b[?1006;2$y", 1004), None);
        assert_eq!(mode(b"\x1b[?62;c", 1004), None);
    }

    #[test]
    fn a_terminal_that_answered_nothing_is_not_known() {
        assert!(answered(b"\x1b[?1004;2$y\x1b[?62;c"));
        assert!(!answered(b""));
        assert!(!answered(b"\x1b[?1004;2$y"));
        assert!(!answered(b"\x1b[?1004;2$y\x1b]99;i=ttyak-probe:p=?;a=focus,report:c"), "DECRQM's answer and a cut OSC ending in c");
    }

    #[test]
    fn a_missing_da1_keeps_what_the_picture_query_found() {
        // Windows Terminal answered ratatui-image's query with sixel, whatever came of DA1.
        assert_eq!(heard(Some(true), false, b""), (Some(true), None));
        assert_eq!(heard(Some(true), false, b"\x1b[?61;4c"), (Some(true), Some(false)));
        // Half blocks only says no only when the terminal was heard at all.
        assert_eq!(heard(Some(false), false, b""), (None, None));
        assert_eq!(heard(Some(false), true, b"\x1b[?1;0c"), (Some(false), Some(true)));
        assert_eq!(heard(None, false, b""), (None, None));
    }

    #[test]
    fn a_console_without_escape_sequences_says_so_and_gets_none() {
        let f = Findings { no_escapes: true, ..sample() };
        let r = report(&f);
        assert!(r.contains("This console does not take escape sequences"), "{r}");
        assert!(!r.contains('\x1b'), "{r:?}");
        assert!(r.contains("Glyphs:"), "{r}");
        assert!(report(&sample()).contains("\x1b]8;;"), "the link sample where escapes work");
    }

    #[test]
    fn colour_from_the_environment() {
        assert_eq!(true_colour(Some("truecolor"), None, false), Some(true));
        assert_eq!(true_colour(Some("24bit"), None, false), Some(true));
        assert_eq!(true_colour(None, None, false), None);
        assert_eq!(true_colour(Some(""), None, false), None);
        assert_eq!(true_colour(Some("truecolor"), None, true), Some(true), "a multiplexer that says so");
    }

    #[test]
    fn windows_terminal_has_true_colour_though_it_sets_no_colorterm() {
        assert_eq!(true_colour(None, Some("8a3f"), false), Some(true));
        assert_eq!(true_colour(Some(""), Some("8a3f"), false), Some(true));
        assert_eq!(true_colour(None, Some(""), false), None, "an empty WT_SESSION says nothing");
        // tmux or screen inside Windows Terminal inherit WT_SESSION, but draw for themselves.
        assert_eq!(true_colour(None, Some("8a3f"), true), None);
        assert!(multiplexer(Some("/tmp/tmux-1000/default,1234,0"), None) && multiplexer(None, Some("1234.pts-0.host")));
        assert!(!multiplexer(None, None) && !multiplexer(Some(""), Some("")), "set but empty says nothing (as osc99::in_tmux)");
    }

    #[test]
    fn utf8_from_the_locale() {
        assert_eq!(utf8(&[Some("en_US.UTF-8")], false), Some(true));
        assert_eq!(utf8(&[None, Some("C.utf8")], false), Some(true));
        assert_eq!(utf8(&[Some(""), Some("POSIX")], false), Some(false));
        assert_eq!(utf8(&[None, None], false), None);
        // Windows: the locale variables say nothing about how ttyak writes there.
        assert_eq!(utf8(&[None, None], true), Some(true));
        assert_eq!(utf8(&[Some("POSIX")], true), Some(true));
    }
}
