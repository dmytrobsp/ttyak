//! Attention: what each incoming event is worth (nothing, unread, for you), the badge state (○ ● ◆) that follows
//! what is unread right now, and which outputs an event fires once the gates let it through.

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

impl Key {
    /// F6's test notification's: no conversation's, so it replaces only itself.
    pub fn test() -> Key {
        Key { workspace: usize::MAX, channel: "ttyak-test".into() }
    }
}

/// What a for-you event shows, and where a click on it lands.
#[derive(Debug, Clone, PartialEq)]
pub struct Note {
    pub key: Key,
    pub title: String,
    pub body: String,
    pub target: Target,
    /// It gets through a pause: a VIP's, as Slack's "Always allow notifications from VIPs" lets through ([`vip_passes`]).
    pub vip: bool,
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
    let vip = vip_passes(ws, m, conv.is_some_and(|c| c.is_dm()));
    Decision::ForYou(Note { key, title, body, target: Target::message(channel, m), vip })
}

/// Whether message `m` gets through a pause, as in Slack's client: "Always allow notifications from VIPs" is on, the
/// plan is paid, and either it is a bot message whose `bot_profile.user_id` is a VIP, or its sender is a VIP and it is a
/// DM or group DM (`is_dm`) or mentions a person, a user group or a channel (`@here`, `@channel` and `@everyone` don't
/// count; the mention needn't be of you). Only the pause: the level, mutes and the other rules still decide first.
pub fn vip_passes(ws: &Workspace, m: &Message, is_dm: bool) -> bool {
    if !ws.paid || !ws.vip_override {
        return false;
    }
    let vip = |u: Option<&str>| u.is_some_and(|u| ws.vip.contains(u));
    let bot = m.subtype.as_deref() == Some("bot_message") && vip(m.bot_profile.as_ref().and_then(|b| b["user_id"].as_str()));
    bot || (vip(m.user.as_deref()) && (is_dm || has_mention(m)))
}

/// Whether `m` mentions a person, a user group or a channel, in its blocks or its text (not `@here` and the like).
fn has_mention(m: &Message) -> bool {
    use format::{Block, Inline};
    let mut blocks = format::from_rich_text(&m.blocks).unwrap_or_default().into_iter().chain(format::from_mrkdwn(&m.text));
    let mention = |i: &Inline| matches!(i, Inline::User { .. } | Inline::Usergroup { .. } | Inline::Channel { .. });
    blocks.any(|b| match b {
        Block::Paragraph(i) | Block::Quote(i) => i.iter().any(mention),
        Block::List { items, .. } => items.iter().flatten().any(mention),
        Block::Code(_) => false,
    })
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
    // (Slack's VIP rule is for messages: a huddle start stays paused.)
    Decision::ForYou(Note { key, title, body: format!("Press {join_key} in ttyak to join"), target, vip: false })
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
    /// The terminal never reported focus, nor said at start that it reports it.
    Unknown,
}

/// What holds back every output of a for-you event.
#[derive(Clone, Copy, Debug)]
pub struct Gates {
    /// The pause or the schedule's quiet time.
    pub paused: bool,
    /// The system's do-not-disturb (where ttyak can read it). Never set by the callers: asking the system takes a
    /// moment, so the notifier's worker asks (`notify::Guard`) and fires nothing while it is on.
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

impl Switches {
    /// The switches in the settings.
    pub fn of(n: &super::settings::Notifications) -> Switches {
        Switches { desktop: n.desktop, sound: n.sound != "off", bell: n.bell }
    }
}

/// What a for-you event fires.
#[derive(Debug, PartialEq, Default)]
pub struct Outputs {
    pub bubble: bool,
    pub sound: bool,
    pub bell: bool,
}

/// What decision `d` fires: nothing unless it is for you and no gate holds it back (a VIP's note gets through the
/// pause, never the system's DND or viewing); then the switches pick the bubble, its sound (no bubble, no sound) and
/// the bell (only while the terminal is known to be unfocused).
pub fn outputs(d: &Decision, g: &Gates, s: &Switches) -> Outputs {
    let Decision::ForYou(note) = d else { return Outputs::default() };
    if (g.paused && !note.vip) || g.system_dnd || g.viewing {
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

    const PAUSED: Gates = Gates { paused: true, ..OPEN };

    /// Bob (U2) is a VIP, "Always allow notifications from VIPs" is on, the plan is paid; Carol (U3) is no VIP.
    fn vip_ws() -> Workspace {
        let mut w = ws();
        w.vip.insert("U2".into());
        w.vip_override = true;
        w.paid = true;
        w.conversations.insert("D3".into(), Conversation { id: "D3".into(), is_im: true, user: Some("U3".into()), ..Default::default() });
        w.conversations.insert("G1".into(), Conversation { id: "G1".into(), is_mpim: true, is_member: true, ..Default::default() });
        w
    }

    /// Whether message `m` in `channel` shows a bubble behind gates `g`.
    fn bubbles(w: &Workspace, channel: &str, m: &Message, g: &Gates) -> bool {
        outputs(&decide(w, 0, channel, m), g, &ALL_ON).bubble
    }

    #[test]
    fn vip_dm_gets_through_the_pause_with_the_override() {
        let w = vip_ws();
        assert_eq!(outputs(&decide(&w, 0, "D1", &msg("U2", "hi")), &PAUSED, &ALL_ON), Outputs { bubble: true, sound: true, bell: true });
        assert!(bubbles(&w, "G1", &msg("U2", "hi all"), &PAUSED), "a group DM too");
        assert!(!bubbles(&w, "D3", &msg("U3", "hi"), &PAUSED), "no VIP: paused");
        assert!(vip_passes(&w, &msg("U2", "hi"), true));
    }

    #[test]
    fn vip_mention_gets_through() {
        let mut w = vip_ws();
        assert!(bubbles(&w, "C1", &msg("U2", "hey <@UME>"), &PAUSED));
        // At the default level a mention of someone else is not for you: nothing to let through.
        assert!(!bubbles(&w, "C1", &msg("U2", "ask <@U3>"), &PAUSED));
        // At "all new messages", any mention lifts the pause, not only one of you.
        w.notify.channels.insert("C1".into(), "everything".into());
        for text in ["ask <@U3>", "ping <!subteam^S1|@devs>", "see <#C9|random>"] {
            assert!(bubbles(&w, "C1", &msg("U2", text), &PAUSED), "{text}");
        }
        let mut rich = msg("U2", "");
        rich.blocks = vec![serde_json::json!({"type": "rich_text", "elements": [{"type": "rich_text_section", "elements": [{"type": "text", "text": "ask "}, {"type": "user", "user_id": "U3"}]}]})];
        assert!(bubbles(&w, "C1", &rich, &PAUSED), "a mention in the blocks");
    }

    #[test]
    fn vip_plain_channel_message_stays_paused() {
        let mut w = vip_ws();
        w.notify.channels.insert("C1".into(), "everything".into());
        assert!(bubbles(&w, "C1", &msg("U2", "plain chatter"), &OPEN));
        assert!(!bubbles(&w, "C1", &msg("U2", "plain chatter"), &PAUSED));
        assert!(!bubbles(&w, "C1", &msg("U2", "`<@U3>` in code"), &PAUSED), "code is no mention");
    }

    #[test]
    fn here_and_channel_do_not_count() {
        let w = vip_ws();
        for text in ["<!here> lunch", "<!channel> lunch", "<!everyone> lunch"] {
            assert!(bubbles(&w, "C1", &msg("U2", text), &OPEN), "{text}: for you");
            assert!(!bubbles(&w, "C1", &msg("U2", text), &PAUSED), "{text}: but no VIP mention");
        }
        let mut rich = msg("U2", "<!here>");
        rich.blocks = vec![serde_json::json!({"type": "rich_text", "elements": [{"type": "rich_text_section", "elements": [{"type": "broadcast", "range": "here"}]}]})];
        assert!(!bubbles(&w, "C1", &rich, &PAUSED));
    }

    #[test]
    fn override_off_holds_vips_back() {
        let w = Workspace { vip_override: false, ..vip_ws() };
        assert!(!bubbles(&w, "D1", &msg("U2", "hi"), &PAUSED));
        assert!(bubbles(&w, "D1", &msg("U2", "hi"), &OPEN));
    }

    #[test]
    fn free_plan_holds_vips_back() {
        let w = Workspace { paid: false, ..vip_ws() };
        assert!(!bubbles(&w, "D1", &msg("U2", "hi"), &PAUSED));
    }

    #[test]
    fn vip_bot_by_its_user_id() {
        let mut w = vip_ws();
        w.vip.insert("UBOT".into());
        w.conversations.insert("D9".into(), Conversation { id: "D9".into(), is_im: true, user: Some("UBOT".into()), ..Default::default() });
        let bot = |user_id: &str| Message {
            ts: "10.0".into(),
            subtype: Some("bot_message".into()),
            bot_id: Some("B1".into()),
            text: "deploy done".into(),
            bot_profile: Some(serde_json::json!({"id": "B1", "name": "deploys", "user_id": user_id})),
            ..Default::default()
        };
        assert!(bubbles(&w, "D9", &bot("UBOT"), &PAUSED));
        assert!(!bubbles(&w, "D9", &bot("UOTHER"), &PAUSED), "a bot that is no VIP");
        // As Slack's client: a VIP bot's message passes without a DM or a mention.
        w.notify.channels.insert("C1".into(), "everything".into());
        assert!(bubbles(&w, "C1", &bot("UBOT"), &PAUSED), "a plain bot message in a channel");
        // Only a `bot_message` counts its `bot_profile`.
        let other = Message { subtype: None, ..bot("UBOT") };
        assert!(!bubbles(&w, "D9", &other, &PAUSED), "not a bot message");
    }

    #[test]
    fn vip_in_muted_channel_stays_quiet() {
        let mut w = vip_ws();
        w.muted.insert("C1".into());
        assert_eq!(decide(&w, 0, "C1", &msg("U2", "hey <@UME>")), Decision::Nothing, "decide unchanged");
        assert!(!bubbles(&w, "C1", &msg("U2", "hey <@UME>"), &PAUSED));
        assert!(!bubbles(&w, "C1", &msg("U2", "hey <@UME>"), &OPEN));
    }

    #[test]
    fn vip_at_nothing_level_stays_quiet() {
        let mut w = vip_ws();
        w.notify.global = "nothing".into();
        assert_eq!(decide(&w, 0, "C1", &msg("U2", "hey <@UME>")), Decision::Unread, "decide unchanged");
        assert!(!bubbles(&w, "C1", &msg("U2", "hey <@UME>"), &PAUSED));
        assert!(!bubbles(&w, "D1", &msg("U2", "hi"), &PAUSED));
    }

    #[test]
    fn system_dnd_still_holds_vips() {
        let w = vip_ws();
        let d = decide(&w, 0, "D1", &msg("U2", "hi"));
        assert!(matches!(&d, Decision::ForYou(n) if n.vip));
        for g in [Gates { system_dnd: true, ..PAUSED }, Gates { system_dnd: true, ..OPEN }, Gates { viewing: true, ..PAUSED }] {
            assert_eq!(outputs(&d, &g, &ALL_ON), Outputs::default(), "{g:?}");
        }
    }

    #[test]
    fn huddle_starts_stay_paused_for_vips() {
        let w = vip_ws();
        assert!(matches!(decide_huddle(&w, 0, "D1", "U2", false, "F3"), Decision::ForYou(n) if !n.vip));
    }
}
