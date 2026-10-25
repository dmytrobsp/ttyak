//! The terminal's tab title: `○ ttyak - <workspace>`, the read/unread mark first, and the title it had before
//! put back when ttyak ends.

use std::io::{self, Write};

use super::attention::State;

/// The longest title written, in characters (the mark and the app name stay, the workspace is cut).
const MAX_CHARS: usize = 80;

/// The title for `workspace` with the mark of `state`: control characters and bidi marks of the name dropped, cut
/// to [`MAX_CHARS`].
pub fn text(state: State, workspace: &str) -> String {
    let name: String = workspace.chars().filter(|c| !c.is_control() && !is_bidi(*c)).collect();
    format!("{} ttyak - {name}", state.mark()).chars().take(MAX_CHARS).collect()
}

/// Characters that reorder or hide text around them.
fn is_bidi(c: char) -> bool {
    matches!(c, '\u{200e}' | '\u{200f}' | '\u{061c}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
}

/// What ttyak wrote into the terminal's title, so that it writes once per change and puts the old title back.
#[derive(Default)]
pub struct Title {
    /// The terminal's own title is on its stack (XTWINOPS 22).
    pushed: bool,
    last: Option<String>,
}

impl Title {
    /// Sets the title to `text` unless it already is; the terminal's own title is pushed before the first write.
    pub fn set(&mut self, out: &mut impl Write, text: &str) -> io::Result<()> {
        if self.last.as_deref() == Some(text) {
            return Ok(());
        }
        if !self.pushed {
            out.write_all(b"\x1b[22;0t")?;
            self.pushed = true;
        }
        write!(out, "\x1b]2;{text}\x07")?;
        out.flush()?;
        self.last = Some(text.to_owned());
        Ok(())
    }

    /// What ttyak last put into the title.
    pub fn last(&self) -> Option<&str> {
        self.last.as_deref()
    }

    /// The title for `workspace` and `state`, when `on` (`notifications.title_mark`); with it off the title is never
    /// touched.
    pub fn sync(&mut self, out: &mut impl Write, on: bool, state: State, workspace: &str) -> io::Result<()> {
        if on { self.set(out, &text(state, workspace)) } else { Ok(()) }
    }

    /// Puts the terminal's own title back, if ttyak changed it.
    pub fn restore(&self, out: &mut impl Write) {
        if self.pushed {
            let _ = out.write_all(b"\x1b[23;0t");
            let _ = out.flush();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_mark_comes_first() {
        assert_eq!(text(State::Unread, "rslack-testbed"), "● ttyak - rslack-testbed");
        assert_eq!(text(State::ForYou, "w"), "◆ ttyak - w");
    }

    #[test]
    fn names_are_cleaned() {
        assert_eq!(text(State::Read, "a\u{1b}]2;x\u{7}b\u{202e}c"), "○ ttyak - a]2;xbc");
        let long = text(State::ForYou, &"w".repeat(500));
        assert!(long.chars().count() <= 80);
        assert!(long.starts_with("◆ ttyak - w"));
    }

    #[test]
    fn written_once_per_change() {
        let (mut t, mut out) = (Title::default(), Vec::new());
        t.set(&mut out, "○ ttyak - a").unwrap();
        t.set(&mut out, "○ ttyak - a").unwrap();
        let s = String::from_utf8(out.clone()).unwrap();
        assert_eq!(s.matches("\x1b]2;").count(), 1);
        assert!(s.find("\x1b[22;0t").unwrap() < s.find("\x1b]2;").unwrap());
        assert_eq!(s.matches("\x1b[22;0t").count(), 1);
        t.set(&mut out, "● ttyak - a").unwrap();
        assert_eq!(String::from_utf8(out.clone()).unwrap().matches("\x1b]2;").count(), 2);
        assert_eq!(String::from_utf8(out.clone()).unwrap().matches("\x1b[22;0t").count(), 1);
        let mut back = Vec::new();
        t.restore(&mut back);
        assert_eq!(back, b"\x1b[23;0t");
    }

    #[test]
    fn off_leaves_the_title_alone() {
        let (mut t, mut out) = (Title::default(), Vec::new());
        t.sync(&mut out, false, State::ForYou, "a").unwrap();
        t.restore(&mut out);
        assert!(out.is_empty());
        t.sync(&mut out, true, State::ForYou, "a").unwrap();
        assert!(String::from_utf8(out).unwrap().contains("◆ ttyak - a"));
    }
}
