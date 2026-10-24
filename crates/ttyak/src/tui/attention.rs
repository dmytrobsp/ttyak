//! Attention: what each incoming event is worth (nothing, unread, for you), the badge state (○ ● ◆) that follows
//! what is unread right now, and which outputs an event fires once the gates let it through.

// The badge and the outputs are read by the tab title, the notifier and `ctl status`, wired in after this module.
#![allow(dead_code)]

use std::collections::HashMap;

use slack_core::format;
use slack_core::types::Message;

use super::live::mentions_me;
use super::notify::Target;
use super::others::Badge;
use super::state::Workspace;

/// One bubble per conversation: a newer one replaces it.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Key {
    pub workspace: usize,
    pub channel: String,
}

/// What a for-you event shows, and where a click on it lands.
#[derive(Debug, Clone, PartialEq)]
pub struct Note {
    pub key: Key,
    pub title: String,
    pub body: String,
    pub target: Target,
}

/// What an incoming event is worth.
#[derive(Debug, Clone, PartialEq)]
pub enum Decision {
    /// Your own messages, hidden or system ones, muted conversations.
    Nothing,
    /// It counts as unread but does not interrupt.
    Unread,
    /// It interrupts (unless a gate holds it back).
    ForYou(Note),
}

const BODY_MAX: usize = 300;

/// What message `m` in `channel` of workspace `workspace` is worth, by Slack's notification rules: the level,
/// keywords, mentions, DMs, replies in your threads. The pause, the system's DND and viewing are gates
/// ([`outputs`]), not part of it.
pub fn decide(ws: &Workspace, workspace: usize, channel: &str, m: &Message) -> Decision {
    let from_me = m.user.as_deref() == Some(ws.self_id.as_str());
    let system = m.subtype.as_deref().is_some_and(|s| !matches!(s, "thread_broadcast" | "file_share" | "bot_message" | "me_message"));
    if from_me || system || m.hidden || ws.muted.contains(channel) {
        return Decision::Nothing;
    }
    let conv = ws.conversations.get(channel);
    let is_im = conv.is_some_and(|c| c.is_im);
    let in_my_thread = m.is_reply()
        && m.thread_ts.as_deref().and_then(|t| ws.find_message(channel, t)).is_some_and(|p| p.user.as_deref() == Some(ws.self_id.as_str()) || p.reply_users.contains(&ws.self_id));
    let mentioned = mentions_me(ws, m) || ws.notify.keyword_in(&m.text).is_some();
    let wanted = match ws.notify.level(channel, conv.is_some_and(|c| c.is_mpim)) {
        "nothing" => false,
        // Every top-level message; replies only where we take part.
        "everything" => is_im || mentioned || in_my_thread || !m.is_reply(),
        _ => is_im || mentioned || in_my_thread || conv.is_some_and(|c| c.is_mpim),
    };
    if !wanted {
        // A reply in a thread you are not in turns nothing unread (unless it is also sent to the channel).
        let elsewhere = m.is_reply() && !in_my_thread && m.subtype.as_deref() != Some("thread_broadcast");
        return if elsewhere { Decision::Nothing } else { Decision::Unread };
    }
    let author = ws.author(m);
    let mut title = match conv {
        Some(c) if c.is_im => author,
        Some(c) if c.is_mpim => format!("{author} in {}", ws.conversation_label(c)),
        _ => format!("{author} in #{}", ws.channel_label(channel)),
    };
    if m.is_reply() {
        title.push_str(" (thread)");
    }
    let mut body = format::plain_text(&format::message_blocks(&m.text, &m.blocks), &|i| super::text::inline_plain(ws, i));
    if body.trim().is_empty() {
        if let Some(f) = m.files.first() {
            body = format!("sent a file: {}", if f.title.is_empty() { &f.name } else { &f.title });
        }
    }
    if body.chars().count() > BODY_MAX {
        body = body.chars().take(BODY_MAX).collect::<String>() + "…";
    }
    let key = Key { workspace, channel: channel.to_string() };
    Decision::ForYou(Note { key, title, body, target: Target::message(channel, m) })
}

/// What `user` starting a huddle in `channel` is worth: for you in a DM or group DM with us (Slack rings there),
/// nothing in channels, muted conversations, or a huddle we are already in (`in_it`). `join_key` goes in the body.
pub fn decide_huddle(ws: &Workspace, workspace: usize, channel: &str, user: &str, in_it: bool, join_key: &str) -> Decision {
    let Some(conv) = ws.conversations.get(channel) else { return Decision::Nothing };
    if in_it || user == ws.self_id || !conv.is_dm() || ws.muted.contains(channel) {
        return Decision::Nothing;
    }
    let who = ws.user_name(user);
    let title = if conv.is_mpim { format!("{who} started a huddle in {}", ws.conversation_label(conv)) } else { format!("{who} started a huddle") };
    let key = Key { workspace, channel: channel.to_string() };
    let target = Target { channel: channel.to_string(), ts: None, thread_ts: None };
    Decision::ForYou(Note { key, title, body: format!("Press {join_key} in ttyak to join"), target })
}

/// The badge: what is unread right now, in every signed-in workspace.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum State {
    Read,
    Unread,
    ForYou,
}

impl State {
    /// The mark in the tab title.
    pub fn mark(self) -> char {
        match self {
            State::Read => '○',
            State::Unread => '●',
            State::ForYou => '◆',
        }
    }

    /// The name in `ttyak ctl status`.
    pub fn name(self) -> &'static str {
        match self {
            State::Read => "read",
            State::Unread => "unread",
            State::ForYou => "for_you",
        }
    }
}

/// The state from Slack's own counts, as the sidebar shows them (`ws`) and the other workspaces' badges (`others`):
/// for you with an unread DM or group DM or a mention (a muted conversation's mentions too: its row is badged),
/// unread with anything else unread (muted conversations aside), the Threads and Activity rows included. Never held
/// back by the pause or focus.
pub fn state(ws: &Workspace, others: &HashMap<usize, Badge>) -> State {
    let (unread, mentions) = ws.total_unread();
    if mentions > 0 || ws.dm_unread() || ws.thread_unread.mentions > 0 || others.values().any(|b| b.attention > 0) {
        State::ForYou
    } else if unread > 0 || ws.thread_unread.has_unreads || ws.activity_unread > 0 || others.values().any(|b| b.unread) {
        State::Unread
    } else {
        State::Read
    }
}

/// Whether the terminal has the focus, as its focus reports say.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TermFocus {
    Focused,
    Unfocused,
    /// The terminal never reported focus.
    Unknown,
}

/// What holds back every output of a for-you event.
#[derive(Clone, Copy, Debug)]
pub struct Gates {
    /// The pause or the schedule's quiet time.
    pub paused: bool,
    /// The system's do-not-disturb (where ttyak can read it).
    pub system_dnd: bool,
    /// The user is looking at that conversation (or thread) in a focused terminal.
    pub viewing: bool,
    pub focus: TermFocus,
}

/// The outputs the user switched on.
#[derive(Clone, Copy, Debug)]
pub struct Switches {
    /// Desktop notifications (the bubble).
    pub desktop: bool,
    /// The notification sound is not "off".
    pub sound: bool,
    /// The bell while the terminal is unfocused.
    pub bell: bool,
}

/// What a for-you event fires.
#[derive(Debug, PartialEq, Default)]
pub struct Outputs {
    pub bubble: bool,
    pub sound: bool,
    pub bell: bool,
}

/// What decision `d` fires: nothing unless it is for you and no gate holds it back; then the switches pick the
/// bubble, its sound (no bubble, no sound) and the bell (only while the terminal is known to be unfocused).
pub fn outputs(d: &Decision, g: &Gates, s: &Switches) -> Outputs {
    if !matches!(d, Decision::ForYou(_)) || g.paused || g.system_dnd || g.viewing {
        return Outputs::default();
    }
    Outputs { bubble: s.desktop, sound: s.desktop && s.sound, bell: s.bell && g.focus == TermFocus::Unfocused }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::state::Unread;
    use slack_core::types::{Conversation, User};

    fn ws() -> Workspace {
        let mut w = Workspace { self_id: "UME".into(), ..Default::default() };
        let mut u = User { id: "U2".into(), name: "bob".into(), ..Default::default() };
        u.profile.display_name = "Bob".into();
        w.users.insert("U2".into(), u);
        w.conversations.insert("D1".into(), Conversation { id: "D1".into(), is_im: true, user: Some("U2".into()), ..Default::default() });
        w.conversations.insert("C1".into(), Conversation { id: "C1".into(), name: "general".into(), is_channel: true, is_member: true, ..Default::default() });
        w
    }

    fn msg(user: &str, text: &str) -> Message {
        Message { ts: "10.0".into(), user: Some(user.into()), text: text.into(), ..Default::default() }
    }

    #[test]
    fn marks() {
        assert_eq!((State::Read.mark(), State::Unread.mark(), State::ForYou.mark()), ('○', '●', '◆'));
        assert_eq!((State::Read.name(), State::Unread.name(), State::ForYou.name()), ("read", "unread", "for_you"));
    }

    #[test]
    fn one_decision_per_message() {
        let mut w = ws();
        let note = match decide(&w, 1, "D1", &msg("U2", "hi")) {
            Decision::ForYou(n) => n,
            d => panic!("{d:?}"),
        };
        assert_eq!(note.key, Key { workspace: 1, channel: "D1".into() });
        assert_eq!((note.title.as_str(), note.body.as_str()), ("Bob", "hi"));
        assert_eq!(note.target, Target::message("D1", &msg("U2", "hi")));
        assert!(matches!(decide(&w, 0, "C1", &msg("U2", "hey <@UME>")), Decision::ForYou(_)));
        assert_eq!(decide(&w, 0, "C1", &msg("U2", "plain chatter")), Decision::Unread);
        assert_eq!(decide(&w, 0, "D1", &msg("UME", "mine")), Decision::Nothing);
        let mut join = msg("U2", "joined");
        join.subtype = Some("channel_join".into());
        assert_eq!(decide(&w, 0, "C1", &join), Decision::Nothing);
        let mut elsewhere = msg("U2", "a reply in a thread you are not in");
        elsewhere.thread_ts = Some("1.0".into());
        assert_eq!(decide(&w, 0, "C1", &elsewhere), Decision::Nothing, "nothing of yours turns unread");
        // The pause is a gate now, not part of the decision.
        w.dnd.snooze_until = Some(f64::MAX);
        assert!(matches!(decide(&w, 0, "D1", &msg("U2", "while paused")), Decision::ForYou(_)));
        w.notify.global = "nothing".into();
        assert_eq!(decide(&w, 0, "D1", &msg("U2", "hi")), Decision::Unread, "Slack's level: unread, never for you");
        w.muted.insert("C1".into());
        assert_eq!(decide(&w, 0, "C1", &msg("U2", "<@UME>")), Decision::Nothing);
    }

    #[test]
    fn huddle_starts_decide_too() {
        let mut w = ws();
        let note = match decide_huddle(&w, 2, "D1", "U2", false, "F3") {
            Decision::ForYou(n) => n,
            d => panic!("{d:?}"),
        };
        assert_eq!((note.key.workspace, note.title.as_str(), note.body.as_str()), (2, "Bob started a huddle", "Press F3 in ttyak to join"));
        assert_eq!(note.target, Target { channel: "D1".into(), ts: None, thread_ts: None }, "a click opens the conversation");
        assert_eq!(decide_huddle(&w, 0, "C1", "U2", false, "F3"), Decision::Nothing);
        assert_eq!(decide_huddle(&w, 0, "D1", "U2", true, "F3"), Decision::Nothing);
        assert_eq!(decide_huddle(&w, 0, "D1", "UME", false, "F3"), Decision::Nothing);
        w.dnd.snooze_until = Some(f64::MAX);
        assert!(matches!(decide_huddle(&w, 0, "D1", "U2", false, "F3"), Decision::ForYou(_)), "the pause is a gate");
    }

    #[test]
    fn state_follows_slacks_counts() {
        let none = HashMap::new();
        let unread = |has_unreads, mentions| Unread { has_unreads, mentions };
        let w = ws();
        assert_eq!(state(&w, &none), State::Read);
        let mut c = ws();
        c.unread.insert("C1".into(), unread(true, 0));
        assert_eq!(state(&c, &none), State::Unread);
        let mut d = ws();
        d.unread.insert("D1".into(), unread(true, 1));
        assert_eq!(state(&d, &none), State::ForYou);
        // Slack's `client.counts` may give an unread DM no mention count: still a DM for you.
        d.unread.insert("D1".into(), unread(true, 0));
        assert_eq!(state(&d, &none), State::ForYou);
        let mut muted = ws();
        muted.unread.insert("C1".into(), unread(true, 0));
        muted.muted.insert("C1".into());
        assert_eq!(state(&muted, &none), State::Read, "muted is not unread");
        muted.unread.insert("C1".into(), unread(true, 1));
        assert_eq!(state(&muted, &none), State::ForYou, "but its mentions are badged, as in the sidebar");
        muted.unread.insert("D1".into(), unread(true, 0));
        muted.muted.insert("D1".into());
        muted.unread.remove("C1");
        assert_eq!(state(&muted, &none), State::Read, "a muted DM neither");
        let mut t = ws();
        t.thread_unread.has_unreads = true;
        assert_eq!(state(&t, &none), State::Unread, "the Threads row");
        t.thread_unread.mentions = 1;
        assert_eq!(state(&t, &none), State::ForYou);
        let mut a = ws();
        a.activity_unread = 2;
        assert_eq!(state(&a, &none), State::Unread, "the Activity row");
        assert_eq!(state(&w, &HashMap::from([(1, Badge { attention: 0, unread: true })])), State::Unread);
        assert_eq!(state(&w, &HashMap::from([(1, Badge { attention: 0, unread: false }), (2, Badge { attention: 2, unread: true })])), State::ForYou);
        assert_eq!(state(&w, &HashMap::from([(1, Badge::default())])), State::Read);
    }

    fn for_you() -> Decision {
        decide(&ws(), 0, "D1", &msg("U2", "hi"))
    }

    const ALL_ON: Switches = Switches { desktop: true, sound: true, bell: true };
    const OPEN: Gates = Gates { paused: false, system_dnd: false, viewing: false, focus: TermFocus::Unfocused };

    #[test]
    fn gates_hold_back_everything() {
        assert_eq!(outputs(&for_you(), &OPEN, &ALL_ON), Outputs { bubble: true, sound: true, bell: true });
        for g in [Gates { paused: true, ..OPEN }, Gates { system_dnd: true, ..OPEN }, Gates { viewing: true, ..OPEN }] {
            assert_eq!(outputs(&for_you(), &g, &ALL_ON), Outputs::default(), "{g:?}");
        }
    }

    #[test]
    fn switches_pick_the_outputs() {
        let bell_only = Switches { desktop: false, sound: true, bell: true };
        assert_eq!(outputs(&for_you(), &OPEN, &bell_only), Outputs { bubble: false, sound: false, bell: true }, "no bubble, no sound");
        let focused = Gates { focus: TermFocus::Focused, ..OPEN };
        assert_eq!(outputs(&for_you(), &focused, &ALL_ON), Outputs { bubble: true, sound: true, bell: false });
        assert_eq!(outputs(&for_you(), &OPEN, &Switches { sound: false, ..ALL_ON }), Outputs { bubble: true, sound: false, bell: true });
        assert_eq!(outputs(&for_you(), &OPEN, &Switches { bell: false, ..ALL_ON }), Outputs { bubble: true, sound: true, bell: false });
        let unknown = Gates { focus: TermFocus::Unknown, ..OPEN };
        assert!(!outputs(&for_you(), &unknown, &ALL_ON).bell, "a terminal that never reported focus: no bell");
        for d in [Decision::Unread, Decision::Nothing] {
            assert_eq!(outputs(&d, &OPEN, &ALL_ON), Outputs::default(), "{d:?}");
        }
    }
}
