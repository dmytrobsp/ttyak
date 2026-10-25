//! The terminal's tab title: `○ ttyak - <workspace>`, the read/unread mark first, and the title it had before
//! put back when ttyak ends (where the terminal keeps a title stack; elsewhere the title is cleared).

use std::io::{self, Write};

use unicode_width::UnicodeWidthChar;

use super::attention::State;

/// The widest title written, in terminal cells (the mark and the app name stay, the workspace is cut).
const MAX_WIDTH: usize = 80;

/// The title for `workspace` with the mark of `state`: control characters and bidi marks of the name dropped, cut
/// to [`MAX_WIDTH`] cells (a wide character that would cross it is left out).
pub fn text(state: State, workspace: &str) -> String {
    let mut width = 0;
    format!("{} ttyak - {}", state.mark(), clean(workspace))
        .chars()
        .take_while(|c| {
            width += c.width().unwrap_or(0);
            width <= MAX_WIDTH
        })
        .collect()
}

/// `name` without control characters and bidi marks.
pub fn clean(name: &str) -> String {
    name.chars().filter(|c| !c.is_control() && !is_bidi(*c)).collect()
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

    /// The title for `workspace` and `state`, when `on` (`notifications.title_mark`); with it off the title is not
    /// touched, and one ttyak set is put back at once ([`Title::restore`]).
    pub fn sync(&mut self, out: &mut impl Write, on: bool, state: State, workspace: &str) -> io::Result<()> {
        if on {
            self.set(out, &text(state, workspace))
        } else {
            self.restore(out);
            Ok(())
        }
    }

    /// Puts the terminal's own title back, if ttyak changed it, once: the title is cleared first (for terminals
    /// without a title stack), then popped (where there is one, the pop wins).
    pub fn restore(&mut self, out: &mut impl Write) {
        if self.pushed {
            let _ = out.write_all(b"\x1b]2;\x07\x1b[23;0t");
            let _ = out.flush();
            (self.pushed, self.last) = (false, None);
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
        // The title cleared first (terminals without a title stack), then the pop (it wins where there is one).
        assert_eq!(back, b"\x1b]2;\x07\x1b[23;0t");
    }

    #[test]
    fn restored_once() {
        let (mut t, mut out) = (Title::default(), Vec::new());
        t.set(&mut out, "○ ttyak - a").unwrap();
        let mut back = Vec::new();
        t.restore(&mut back);
        assert!(!back.is_empty());
        let mut again = Vec::new();
        t.restore(&mut again);
        assert!(again.is_empty(), "the old title is popped only once: {again:?}");
    }

    #[test]
    fn cut_by_display_width() {
        use unicode_width::UnicodeWidthStr;
        let wide = text(State::Unread, &"界".repeat(100));
        assert!(wide.width() <= 80, "{} cells", wide.width());
        assert!(wide.width() >= 79, "filled up to the last wide character: {} cells", wide.width());
        assert!(wide.starts_with("● ttyak - 界"));
    }

    #[test]
    fn switched_off_while_running_puts_the_old_title_back_at_once() {
        let (mut t, mut out) = (Title::default(), Vec::new());
        t.sync(&mut out, true, State::Unread, "a").unwrap();
        let mut off = Vec::new();
        t.sync(&mut off, false, State::Unread, "a").unwrap();
        assert_eq!(off, b"\x1b]2;\x07\x1b[23;0t", "cleared, then popped");
        let mut again = Vec::new();
        t.sync(&mut again, false, State::ForYou, "a").unwrap();
        t.restore(&mut again);
        assert!(again.is_empty(), "once: {again:?}");
        // On again: the terminal's title pushed anew, the mark back.
        let mut on = Vec::new();
        t.sync(&mut on, true, State::ForYou, "a").unwrap();
        let on = String::from_utf8(on).unwrap();
        assert!(on.starts_with("\x1b[22;0t") && on.contains("◆ ttyak - a"), "{on:?}");
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
