//! Notifications through the terminal itself (kitty's OSC 99, also foot, Konsole, Contour): the terminal shows the
//! bubble, focuses its own window and tab on a click and reports the click back.

use super::attention::Key;
use super::system_dnd::{self, SoundBy};

/// The id the probe asks under.
const PROBE_ID: &str = "ttyak-probe";

/// The most one title or body chunk may carry before base64 (the protocol's limit).
const CHUNK: usize = 2048;

/// The query sent at start: which keys and payloads the terminal understands.
pub fn probe() -> &'static str {
    "\x1b]99;i=ttyak-probe:p=?;\x1b\\"
}

/// Whether `reply` (what the terminal sent back after [`probe`]) holds an answer that offers `a=focus,report` and
/// `p=title,body`.
pub fn supported(reply: &str) -> bool {
    let head = format!("\x1b]99;i={PROBE_ID}:p=?;");
    let Some(start) = reply.find(&head) else { return false };
    let answer = &reply[start + head.len()..];
    let answer = &answer[..answer.find(['\x1b', '\x07']).unwrap_or(answer.len())];
    let offers = |key: &str, wanted: &[&str]| answer.split(':').filter_map(|kv| kv.strip_prefix(key)).any(|values| wanted.iter().all(|w| values.split(',').any(|v| v == *w)));
    offers("a=", &["focus", "report"]) && offers("p=", &["title", "body"])
}

/// The notification id of `key`'s bubble: `w<index>-<channel>`, only `[A-Za-z0-9_-]` (the terminal echoes it back).
pub fn id(key: &Key) -> String {
    let channel: String = key.channel.chars().filter(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-')).collect();
    format!("w{}-{channel}", key.workspace)
}

/// The sequences that show (or replace, by `id`) a bubble titled `title` saying `body`, with the sound `sound` (kitty's
/// names: `system`, `silent`, …; None: the terminal's default); a click focuses the window and is reported, and so is
/// its closing. The text goes in base64, each cut to what one chunk may carry.
pub fn show(id: &str, title: &str, body: &str, sound: Option<&str>) -> String {
    use super::clipboard::base64;
    let mut meta = format!("i={id}:a=focus,report:c=1:e=1");
    if let Some(sound) = sound {
        meta.push_str(&format!(":s={}", base64(sound.as_bytes())));
    }
    let chunk = |done: u8, kind: &str, text: &str| format!("\x1b]99;{meta}:d={done}:p={kind};{}\x1b\\", base64(cut(text, CHUNK).as_bytes()));
    chunk(0, "title", title) + &chunk(1, "body", body)
}

/// `text` cut to at most `max` bytes, at a character's end.
fn cut(text: &str, max: usize) -> &str {
    let mut end = text.len().min(max);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    &text[..end]
}

/// The sequence that takes bubble `id` away.
pub fn close(id: &str) -> String {
    format!("\x1b]99;i={id}:p=close;\x1b\\")
}

/// `seq` through tmux's passthrough (`allow-passthrough` on), to the terminal outside.
pub fn wrap_for_tmux(seq: &str) -> String {
    format!("\x1bPtmux;{}\x1b\\", seq.replace('\x1b', "\x1b\x1b"))
}

/// What the terminal reported about a bubble.
#[derive(Debug, Clone, PartialEq)]
pub enum Report {
    Clicked(String),
    Closed(String),
}

/// The report in an OSC payload from the terminal (`99;i=<id>;` clicked, also with a button's number after it;
/// `99;i=<id>:p=close;` closed); None for anything else (the probe's answer, `p=alive`, the id 0 of a bubble sent
/// without one).
pub fn report(osc: &str) -> Option<Report> {
    let (meta, _) = osc.strip_prefix("99;")?.split_once(';')?;
    let (mut id, mut kind) = (None, None);
    for kv in meta.split(':') {
        match kv.split_once('=') {
            Some(("i", v)) => id = Some(v),
            Some(("p", v)) => kind = Some(v),
            _ => {}
        }
    }
    let id = id.filter(|i| !i.is_empty() && *i != "0")?.to_string();
    match kind {
        None => Some(Report::Clicked(id)),
        Some("close") => Some(Report::Closed(id)),
        Some(_) => None,
    }
}

/// Who plays `sound` (the settings' value) with the terminal's bubble, given the notification server's `capabilities`:
/// the terminal can only ask for the desktop's own sound (`system`, as the system's route does: kitty passes it to
/// the same server); ttyak plays its own sounds, which the terminal cannot carry.
pub fn sound_by(capabilities: &[String], sound: &str) -> SoundBy {
    match sound {
        "off" => SoundBy::None,
        "system" => system_dnd::sound_route(capabilities, sound),
        _ => SoundBy::Ttyak,
    }
}

/// kitty's sound name for a bubble whose sound `by` plays: the desktop's own, or none.
pub fn sound_name(by: SoundBy) -> &'static str {
    if by == SoundBy::Notification { "system" } else { "silent" }
}

/// Whether tmux lets ttyak's bubbles out now: `allow-passthrough` (`tmux show -gv allow-passthrough`) is `on` or `all`,
/// and `active` (`#{pane_active} #{window_active} #{session_attached}` of ttyak's pane) says the pane is the one shown
/// (the terminal's report reaches the active pane only).
pub fn tmux_allows(passthrough: &str, active: &str) -> bool {
    let fields: Vec<&str> = active.split_whitespace().collect();
    matches!(passthrough.trim(), "on" | "all") && fields.len() == 3 && fields.iter().all(|f| f.parse::<u32>().is_ok_and(|n| n > 0))
}

/// Whether the terminal's notifications may be used: `TTYAK_NOTIFY_ROUTE=system` (for debugging) keeps to the system's.
pub fn wanted() -> bool {
    std::env::var("TTYAK_NOTIFY_ROUTE").map_or(true, |route| route != "system")
}

/// What the terminal is called, for F6 (from what it puts in the environment).
pub fn terminal_name() -> String {
    let var = |name: &str| std::env::var(name).ok().filter(|v| !v.is_empty());
    if var("KITTY_WINDOW_ID").is_some() || var("TERM").is_some_and(|t| t == "xterm-kitty") {
        "kitty".into()
    } else if var("KONSOLE_VERSION").is_some() {
        "Konsole".into()
    } else if var("TERM").is_some_and(|t| t.starts_with("foot")) {
        "foot".into()
    } else {
        var("TERM_PROGRAM").unwrap_or_else(|| "the terminal".into())
    }
}

/// ttyak's tmux pane, when it runs inside tmux.
pub fn tmux_pane() -> Option<String> {
    std::env::var_os("TMUX")?;
    std::env::var("TMUX_PANE").ok().filter(|p| !p.is_empty())
}

/// tmux's `allow-passthrough`, asked at start (before the event loop; empty when tmux does not answer).
pub fn tmux_passthrough() -> String {
    let out = std::process::Command::new("tmux").args(["show", "-gv", "allow-passthrough"]).stdin(std::process::Stdio::null()).stderr(std::process::Stdio::null()).output();
    out.ok().filter(|o| o.status.success()).map(|o| String::from_utf8_lossy(&o.stdout).into_owned()).unwrap_or_default()
}

/// How long tmux is waited for when a bubble is sent (an internal wait).
const TMUX_WAIT: std::time::Duration = std::time::Duration::from_secs(1);

/// Whether tmux lets a bubble from `pane` out right now ([`tmux_allows`]); no answer within [`TMUX_WAIT`] counts as no.
pub async fn tmux_lets_out(pane: &str) -> bool {
    async fn tmux(args: &[&str]) -> String {
        let out = tokio::process::Command::new("tmux").args(args).stdin(std::process::Stdio::null()).stderr(std::process::Stdio::null()).kill_on_drop(true).output().await;
        out.ok().filter(|o| o.status.success()).map(|o| String::from_utf8_lossy(&o.stdout).into_owned()).unwrap_or_default()
    }
    let asked = async { (tmux(&["show", "-gv", "allow-passthrough"]).await, tmux(&["display", "-p", "-t", pane, "#{pane_active} #{window_active} #{session_attached}"]).await) };
    matches!(tokio::time::timeout(TMUX_WAIT, asked).await, Ok((passthrough, active)) if tmux_allows(&passthrough, &active))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// kitty 0.48.2's answer to [`probe`], then its DA1 answer (captured 2026-10-25).
    const KITTY: &str = "\x1b]99;i=ttyak-probe:p=?;a=focus,report:o=always,unfocused,invisible:u=0,1,2:p=title,body,?,close,icon,alive,buttons:c=1:w=1:s=system,silent,error,info,question,warn,warning\x1b\\\x1b[?62;52;c";

    fn key(workspace: usize, channel: &str) -> Key {
        Key { workspace, channel: channel.into() }
    }

    /// The payloads of `kind` (`title`, `body`) in `seq`, decoded.
    fn payload(seq: &str, kind: &str) -> String {
        let mut text = Vec::new();
        for osc in seq.split("\x1b]99;").skip(1) {
            let osc = osc.strip_suffix("\x1b\\").expect("ended by ST");
            let (meta, data) = osc.split_once(';').unwrap();
            assert!(meta.split(':').any(|kv| kv == "e=1"), "{meta}");
            if meta.split(':').any(|kv| kv == format!("p={kind}")) {
                text.extend(unbase64(data));
            }
        }
        String::from_utf8(text).unwrap()
    }

    fn unbase64(data: &str) -> Vec<u8> {
        const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let bits: Vec<u8> = data.bytes().filter(|b| *b != b'=').map(|b| ALPHABET.iter().position(|a| *a == b).unwrap() as u8).collect();
        bits.chunks(4).flat_map(|c| {
            let n = c.iter().enumerate().fold(0u32, |n, (i, v)| n | (*v as u32) << (18 - 6 * i));
            (0..c.len() - 1).map(move |i| (n >> (16 - 8 * i)) as u8)
        }).collect()
    }

    #[test]
    fn ids_are_safe_to_echo() {
        assert_eq!(id(&key(0, "C0123")), "w0-C0123");
        assert_eq!(id(&key(2, "D9;x:y=z\x1b é_-")), "w2-D9xyz_-");
    }

    #[test]
    fn show_carries_the_text_and_asks_for_the_click() {
        let seq = show("w0-C1", "Bob in #general", "hi; ünïcode\nand a line", Some("silent"));
        assert_eq!(payload(&seq, "title"), "Bob in #general");
        assert_eq!(payload(&seq, "body"), "hi; ünïcode\nand a line");
        assert!(seq.contains("a=focus,report") && seq.contains("i=w0-C1"), "{seq:?}");
        // Only printable ASCII inside: no control character of the text reaches the terminal.
        assert!(seq.split("\x1b]99;").skip(1).all(|o| o.trim_end_matches("\x1b\\").bytes().all(|b| (0x20..0x7f).contains(&b))), "{seq:?}");
        // The sound by kitty's name, in base64; the last chunk shows it.
        assert!(seq.contains(&format!("s={}", super::super::clipboard::base64(b"silent"))), "{seq:?}");
        assert!(seq.rsplit("\x1b]99;").next().unwrap().contains("d=1"), "{seq:?}");
        // A long body is cut to what one chunk may carry.
        let long = show("w0-C1", "t", &"é".repeat(5000), None);
        assert!(payload(&long, "body").len() <= 2048 && payload(&long, "body").starts_with("éé"));
        assert!(!long.contains("s="), "no sound name: the terminal's default");
    }

    #[test]
    fn close_takes_it_away() {
        assert_eq!(close("w0-C1"), "\x1b]99;i=w0-C1:p=close;\x1b\\");
    }

    #[test]
    fn reports() {
        assert_eq!(report("99;i=w0-C1;"), Some(Report::Clicked("w0-C1".into())));
        assert_eq!(report("99;i=w0-C1;1"), Some(Report::Clicked("w0-C1".into())), "a button");
        assert_eq!(report("99;i=w0-C1:p=close;"), Some(Report::Closed("w0-C1".into())));
        assert_eq!(report("99;i=w0-C1:p=close;untracked"), Some(Report::Closed("w0-C1".into())));
        for other in ["99;i=w0-C1:p=alive;w0-C1", "99;i=ttyak-probe:p=?;a=focus", "99;i=0;", "99;;", "99;i=w0-C1", "52;c;aGk="] {
            assert_eq!(report(other), None, "{other}");
        }
    }

    #[test]
    fn the_probe_and_its_answer() {
        assert_eq!(probe(), "\x1b]99;i=ttyak-probe:p=?;\x1b\\");
        assert!(supported(KITTY));
        assert!(supported(KITTY.trim_end_matches("\x1b[?62;52;c")), "without the DA1 answer");
        assert!(supported("\x1b]99;i=ttyak-probe:p=?;p=title,body:a=report,focus\x07"), "BEL, any order");
        // foot 1.16 answers DA1 only (captured 2026-10-25).
        for no in ["", "\x1b[?62;4;22c", "\x1b[?62;52;c", "garbage\x1b]99;", "\x1b]99;i=ttyak-probe:p=?;a=focus:p=title,body\x1b\\", "\x1b]99;i=ttyak-probe:p=?;a=focus,report:p=title\x1b\\", "\x1b]99;i=other:p=?;a=focus,report:p=title,body\x1b\\"] {
            assert!(!supported(no), "{no:?}");
        }
    }

    #[test]
    fn tmux_doubles_escape() {
        assert_eq!(wrap_for_tmux("\x1b]99;i=a:p=close;\x1b\\"), "\x1bPtmux;\x1b\x1b]99;i=a:p=close;\x1b\x1b\\\x1b\\");
        assert!(tmux_allows("on\n", "1 1 1\n") && tmux_allows("all", "1 1 2"));
        assert!(!tmux_allows("off", "1 1 1") && !tmux_allows("", "1 1 1"), "no passthrough");
        assert!(!tmux_allows("on", "0 1 1") && !tmux_allows("on", "1 0 1") && !tmux_allows("on", "1 1 0") && !tmux_allows("on", ""), "not the pane shown");
    }

    #[test]
    fn the_terminal_plays_only_the_desktops_sound() {
        let sound = vec!["body".to_string(), "sound".to_string()];
        assert_eq!(sound_by(&sound, "system"), SoundBy::Notification);
        assert_eq!(sound_by(&[], "system"), SoundBy::Ttyak, "nobody: the server has none");
        assert_eq!(sound_by(&sound, "chime"), SoundBy::Ttyak, "kitty cannot play a file");
        assert_eq!(sound_by(&sound, "off"), SoundBy::None);
        assert_eq!((sound_name(SoundBy::Notification), sound_name(SoundBy::Ttyak), sound_name(SoundBy::None)), ("system", "silent", "silent"));
    }
}
