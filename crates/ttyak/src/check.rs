//! `ttyak ctl check`: what the terminal answered and what the system has, and one sample line per kind of glyph for
//! the user to judge (a program cannot tell which font the terminal draws with). No Slack, no network.

/// What was found. A terminal row is yes, no, or None (the terminal did not say); a system row is its value, or None
/// (could not be told).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Findings {
    pub terminal: Vec<(String, Option<bool>)>,
    pub system: Vec<(String, Option<String>)>,
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
const MAC_SERVICE: &str = "macOS Notification Centre";
const WINDOWS_SERVICE: &str = "Windows toasts";
const NO_SERVER: &str = "none answered";
const WAYLAND_CLIPBOARD: &str = "Wayland (needs a compositor with data-control; else the terminal's OSC 52)";
const SYSTEM_CLIPBOARD: &str = "the system's";
// Report.
const TITLE: &str = "ttyak ctl check — what this terminal and system offer";
const TERMINAL: &str = "The terminal";
const SYSTEM: &str = "The system";
const NOT_A_TERMINAL: &str = "Not a terminal (stdin is not one): the terminal part is skipped. Run it in your terminal.";
const GLYPHS_HEAD: &str = "Glyphs: the | at the end of each line should sit in one column";
const LOOK_FOR: &str = "Look for empty boxes or misaligned columns";
const FONTS_HINT: &str = "Boxes: install the fonts listed under Requirements in the README, or turn on F6 → Emoji as names. Misaligned rows: the terminal measures emoji differently.";
const LINKS_NAME: &str = "links (OSC 8)";
const LINK_TEXT: &str = "a link to ttyak.sh";

pub const WORDS: &[&str] = &[
    UTF8, TRUE_COLOUR, MOUSE, FOCUS, PASTE, LINKS, OSC99, PICTURES, NO_PICTURES, SIZE, OPENER, NOTIFIER, CAPABILITIES, DND, SHOWN_BY, SOUND_BY, CLICK, RAISE, FFMPEG, GSTREAMER,
    CLIPBOARD, FOUND, MISSING, BUILT_IN, YES, NO, NOT_KNOWN, MAC_SERVICE, WINDOWS_SERVICE, NO_SERVER, WAYLAND_CLIPBOARD, SYSTEM_CLIPBOARD, TITLE, TERMINAL, SYSTEM, NOT_A_TERMINAL,
    GLYPHS_HEAD, LOOK_FOR, FONTS_HINT, LINKS_NAME, LINK_TEXT, "symbols", "arrows", "boxes and blocks", "emoji", "CJK",
];

/// The terminal's answer about private mode `mode` (DECRQM: `CSI ? mode ; state $ y`): yes when it knows the mode, no
/// when it says it does not, None when it did not answer.
fn mode(reply: &[u8], mode: u32) -> Option<bool> {
    let text = String::from_utf8_lossy(reply);
    let head = format!("\x1b[?{mode};");
    let rest = &text[text.find(&head)? + head.len()..];
    let state = &rest[..rest.find("$y")?];
    match state {
        "1" | "2" | "3" => Some(true),
        "0" | "4" => Some(false),
        _ => None,
    }
}

/// Whether `reply` holds the terminal's answer to DA1 (`CSI ? … c`), which ends the answers it gives: without it the
/// terminal answered nothing, so nothing it did not answer is known.
fn answered(reply: &[u8]) -> bool {
    reply.windows(3).any(|w| w == b"\x1b[?") && reply.ends_with(b"c")
}

/// 24-bit colour from `$COLORTERM`: a terminal that does not say may still have it.
fn true_colour(colorterm: Option<&str>) -> Option<bool> {
    colorterm.filter(|c| matches!(c.trim(), "truecolor" | "24bit")).map(|_| true)
}

/// UTF-8 from the locale variables (`$LC_ALL`, `$LC_CTYPE`, `$LANG`, the first one set decides); None when none is set.
fn utf8(vars: &[Option<&str>]) -> Option<bool> {
    let v = vars.iter().flatten().find(|v| !v.is_empty())?.to_lowercase();
    Some(v.contains("utf-8") || v.contains("utf8"))
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

/// What the terminal answers: queried in raw mode for a moment, before anything else reads the input.
fn terminal() -> (Vec<(String, Option<bool>)>, Option<bool>) {
    use crate::tui::i18n::tr;
    use std::io::IsTerminal;
    if !std::io::stdin().is_terminal() || !std::io::stdout().is_terminal() {
        return (Vec::new(), None);
    }
    let var = |n: &str| std::env::var(n).ok();
    let mut rows = vec![(tr(UTF8).into_owned(), utf8(&[var("LC_ALL").as_deref(), var("LC_CTYPE").as_deref(), var("LANG").as_deref()]))];
    rows.push((tr(TRUE_COLOUR).into_owned(), true_colour(var("COLORTERM").as_deref())));
    let (mut graphics, mut osc99, mut modes) = (None, false, Vec::new());
    if let Some(_raw) = Raw::on() {
        let media = crate::tui::media::Media::detect("auto");
        (graphics, osc99) = (Some(media.graphics()), media.osc99);
        modes = crate::tui::media::device_attributes("\x1b[?1000$p\x1b[?1004$p\x1b[?2004$p");
    }
    rows.push((tr(MOUSE).into_owned(), mode(&modes, 1000)));
    rows.push((tr(FOCUS).into_owned(), mode(&modes, 1004)));
    rows.push((tr(PASTE).into_owned(), mode(&modes, 2004)));
    rows.push((tr(LINKS).into_owned(), None));
    let heard = answered(&modes);
    let (graphics, osc99) = (graphics.filter(|_| heard), heard.then_some(osc99));
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

/// What the terminal answered and what the system has. Nothing leaves the machine.
pub fn findings() -> Findings {
    use crate::tui::{i18n::tr, osc99, raise};
    if let Err(e) = crate::tui::i18n::set(&crate::tui::settings::load().0.region.language) {
        tracing::debug!("language: {e}");
    }
    let (terminal, osc99_answered) = terminal();
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
    Findings { terminal, system }
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
        out += &format!("  {}: \x1b]8;;https://ttyak.sh\x1b\\{}\x1b]8;;\x1b\\\n", tr(LINKS_NAME), tr(LINK_TEXT));
        out += &format!("\n{}. {}\n", tr(LOOK_FOR), tr(FONTS_HINT));
    }
    out
}

pub fn run() {
    print!("{}", report(&findings()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::i18n::tr;

    fn sample() -> Findings {
        Findings {
            terminal: vec![("UTF-8 text".into(), Some(true)), ("mouse reporting".into(), Some(false)), ("focus reporting".into(), None)],
            system: vec![("link opener".into(), Some("found".into())), ("ffmpeg".into(), None)],
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
        let r = report(&Findings { terminal: vec![], system: vec![("ffmpeg".into(), Some("found".into()))] });
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
    }

    #[test]
    fn colour_from_the_environment() {
        assert_eq!(true_colour(Some("truecolor")), Some(true));
        assert_eq!(true_colour(Some("24bit")), Some(true));
        assert_eq!(true_colour(None), None);
        assert_eq!(true_colour(Some("")), None);
    }

    #[test]
    fn utf8_from_the_locale() {
        assert_eq!(utf8(&[Some("en_US.UTF-8")]), Some(true));
        assert_eq!(utf8(&[None, Some("C.utf8")]), Some(true));
        assert_eq!(utf8(&[Some(""), Some("POSIX")]), Some(false));
        assert_eq!(utf8(&[None, None]), None);
    }
}
