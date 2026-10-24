//! Desktop notifications for incoming messages: one bubble per conversation, withdrawn once read, and the bell.

use std::collections::HashMap;
use std::future::Future;
use std::io::{self, Write};
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::{Arc, OnceLock};

use slack_core::types::Message;
use tokio::sync::mpsc;

use super::attention::{self, Decision, Key, Note, Outputs, TermFocus};
use super::state::Workspace;

/// Title and body for a notification about `m` in `channel`, or None if it should stay
/// quiet. `viewing`: the user is looking at this conversation (or its thread) right now.
#[cfg(test)] // the old rules' tests; the app decides with `attention`
pub fn notification(ws: &Workspace, channel: &str, m: &Message, viewing: bool, now: f64) -> Option<(String, String)> {
    if viewing || ws.dnd_active(now) {
        return None;
    }
    shown(attention::decide(ws, 0, channel, m))
}

/// Notification for someone starting a huddle in a DM or group DM with us (Slack rings
/// there); None for channels, muted conversations, DND, or a huddle we are already in.
pub fn huddle_notification(ws: &Workspace, channel: &str, user: &str, in_it: bool, now: f64, join_key: &str) -> Option<(String, String)> {
    if ws.dnd_active(now) {
        return None;
    }
    shown(attention::decide_huddle(ws, 0, channel, user, in_it, join_key))
}

/// The title and body of a for-you decision.
fn shown(d: Decision) -> Option<(String, String)> {
    match d {
        Decision::ForYou(n) => Some((n.title, n.body)),
        Decision::Unread | Decision::Nothing => None,
    }
}

const NEW_VERSION: &str = "ttyak {version} is out";
const YOU_HAVE: &str = "You have {current}. See F6 → Updates.";
/// The update notice's words, for the locale template.
pub const UPDATE_WORDS: &[&str] = &[NEW_VERSION, YOU_HAVE];

/// Title and body for a newer ttyak, `latest`, when this one is `current`.
pub fn update_notification(latest: &crate::update::Latest, current: &str) -> (String, String) {
    use super::i18n::tr;
    (tr(NEW_VERSION).replace("{version}", &latest.version.to_string()), tr(YOU_HAVE).replace("{current}", current))
}

/// Where clicking a notification takes you: a conversation, and the message in it.
#[derive(Debug, Clone, PartialEq)]
pub struct Target {
    pub channel: String,
    pub ts: Option<String>,
    /// The thread the message is in.
    pub thread_ts: Option<String>,
}

impl Target {
    pub fn message(channel: &str, m: &Message) -> Target {
        Target { channel: channel.to_string(), ts: Some(m.ts.clone()), thread_ts: m.thread_ts.clone().filter(|_| m.is_reply()) }
    }
}

/// A notification titled `title` saying `body`, with the settings' `sound` ("system": the
/// desktop's); None: none at all.
fn notice(title: &str, body: &str, sound: Option<&str>) -> notify_rust::Notification {
    let mut n = notify_rust::Notification::new();
    n.appname("ttyak").summary(title).body(body);
    match sound {
        Some("system") => {
            n.sound_name(super::sound::system_sound_name());
        }
        Some(_) => {}
        // (Without a sound name macOS and Windows stay silent; XDG servers may play their own.)
        #[cfg(all(unix, not(target_os = "macos")))]
        None => {
            n.hint(notify_rust::Hint::SuppressSound(true));
        }
        #[cfg(not(all(unix, not(target_os = "macos"))))]
        None => {}
    }
    n
}

/// How long a click on a notification still counts (the notification centre keeps them).
const CLICK_WAIT: std::time::Duration = std::time::Duration::from_secs(30 * 60);

/// A future a notification backend hands back.
type Fut<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// What a bubble shows.
#[derive(Debug, Clone)]
struct Bubble {
    title: String,
    body: String,
    /// The settings' sound for the server ("system": the desktop's); None: silent.
    sound: Option<String>,
    /// The server's id of the bubble it replaces.
    replaces: Option<u32>,
    /// A click is waited for.
    clickable: bool,
}

/// Where bubbles are shown: the system's notifier (tests use one in memory).
trait Backend: Send + Sync {
    fn show(&self, bubble: Bubble) -> Fut<'static, anyhow::Result<Arc<dyn Shown>>>;
}

/// A bubble on screen.
trait Shown: Send + Sync {
    /// The server's id.
    fn id(&self) -> u32;
    /// Until it is clicked (true) or closed (false).
    fn clicked(&self) -> Fut<'_, bool>;
    /// Take it off the screen.
    fn close(&self) -> Fut<'_, ()>;
}

/// The system's notifier through notify-rust: freedesktop's notification server (replacing, closing, clicks), or
/// macOS's and Windows' (shown only, for now).
struct System;

impl Backend for System {
    fn show(&self, bubble: Bubble) -> Fut<'static, anyhow::Result<Arc<dyn Shown>>> {
        Box::pin(async move {
            #[cfg(all(unix, not(target_os = "macos")))]
            {
                let mut n = notice(&bubble.title, &bubble.body, bubble.sound.as_deref());
                if let Some(id) = bubble.replaces {
                    n.id(id);
                }
                // A click invokes the "default" action, which servers only offer when it is declared.
                if bubble.clickable {
                    n.action("default", "Open");
                }
                let handle = n.show_async().await?;
                tracing::debug!(id = handle.id(), replaced = bubble.replaces.is_some(), "notification shown");
                Ok(Arc::new(Xdg(handle)) as Arc<dyn Shown>)
            }
            #[cfg(not(all(unix, not(target_os = "macos"))))]
            {
                let n = notice(&bubble.title, &bubble.body, bubble.sound.as_deref());
                let _ = (bubble.replaces, bubble.clickable);
                tokio::task::spawn_blocking(move || {
                    if let Err(e) = n.show() {
                        tracing::warn!("notification failed: {e}");
                    }
                });
                Ok(Arc::new(Unwatched) as Arc<dyn Shown>)
            }
        })
    }
}

/// A bubble of the freedesktop notification server.
#[cfg(all(unix, not(target_os = "macos")))]
struct Xdg(notify_rust::NotificationHandle);

#[cfg(all(unix, not(target_os = "macos")))]
impl Shown for Xdg {
    fn id(&self) -> u32 {
        self.0.id()
    }

    fn clicked(&self) -> Fut<'_, bool> {
        Box::pin(async move {
            let mut clicked = false;
            self.0.wait_for_action_async(|r| clicked = matches!(r, notify_rust::NotificationResponse::Default)).await;
            clicked
        })
    }

    fn close(&self) -> Fut<'_, ()> {
        Box::pin(self.0.close_async())
    }
}

/// A bubble ttyak hears nothing more of (a click does what the system does).
#[cfg(not(all(unix, not(target_os = "macos"))))]
struct Unwatched;

#[cfg(not(all(unix, not(target_os = "macos"))))]
impl Shown for Unwatched {
    fn id(&self) -> u32 {
        0
    }

    fn clicked(&self) -> Fut<'_, bool> {
        Box::pin(async { false })
    }

    fn close(&self) -> Fut<'_, ()> {
        Box::pin(async {})
    }
}

/// The process's notifications, for every workspace: one bubble per conversation (a newer one replaces it), withdrawn
/// once what it shows is read, one click waiter per bubble; and the bell, rung from any workspace and written by the run
/// loop between draws. Bubbles are shown, replaced and closed in order by one worker.
#[derive(Clone)]
pub struct Notifier {
    inner: Arc<Inner>,
}

struct Inner {
    backend: Arc<dyn Backend>,
    /// The worker's queue, started with the first request.
    queue: OnceLock<mpsc::UnboundedSender<Cmd>>,
    /// A bell is due.
    bell: AtomicBool,
    /// The terminal's focus as last reported (the other workspaces' watchers share the window).
    focus: AtomicU8,
}

/// A request to the worker.
enum Cmd {
    /// Show a bubble; `key` None: one no other replaces or read withdraws (the update notice).
    Show { key: Option<Key>, at: Option<Place>, bubble: Bubble, on_click: Option<Box<dyn FnOnce() + Send>> },
    /// Close `key`'s bubble: always, or (`read`) only when that read covers what it shows.
    Withdraw { key: Key, read: Option<Place> },
    /// The waiter of `key`'s bubble `generation` ended; `gone`: it was clicked or closed.
    Ended { key: Key, generation: u64, gone: bool },
    #[cfg(test)]
    Settled(tokio::sync::oneshot::Sender<()>),
}

/// Where a bubble's message is, or how far a read went: the thread (None: the conversation itself) and the ts.
#[derive(Debug, Clone, PartialEq)]
struct Place {
    thread: Option<String>,
    ts: String,
}

impl Place {
    /// Whether read `read` covers the message here.
    fn read_by(&self, read: &Place) -> bool {
        self.thread == read.thread && matches!((ts_order(&self.ts), ts_order(&read.ts)), (Some(a), Some(b)) if a <= b)
    }
}

/// A Slack ts ("1792882650.243419") in order: seconds, then microseconds.
fn ts_order(ts: &str) -> Option<(u64, u64)> {
    let (secs, micros) = ts.split_once('.').unwrap_or((ts, "0"));
    Some((secs.parse().ok()?, format!("{micros:0<6}").get(..6)?.parse().ok()?))
}

/// A bubble on screen.
struct Live {
    shown: Arc<dyn Shown>,
    /// The message it shows (None: a huddle's start).
    at: Option<Place>,
    generation: u64,
    /// Its click waiter, while it waits.
    waiter: Option<tokio::task::AbortHandle>,
}

/// How notifications reach the user on this machine, for F6's "On this machine" (Tasks 8–12 fill in their routes).
#[allow(dead_code)] // read by F6, wired in after this
#[derive(Debug, Clone, PartialEq)]
pub struct RouteInfo {
    pub shown_by: String,
    pub sound_by: String,
    pub click: String,
}

impl Notifier {
    /// Notifications through the system's notifier.
    pub fn new() -> Notifier {
        Notifier::with(Arc::new(System))
    }

    fn with(backend: Arc<dyn Backend>) -> Notifier {
        Notifier { inner: Arc::new(Inner { backend, queue: OnceLock::new(), bell: AtomicBool::new(false), focus: AtomicU8::new(TermFocus::Unknown as u8) }) }
    }

    fn send(&self, cmd: Cmd) {
        let queue = self.inner.queue.get_or_init(|| {
            let (tx, rx) = mpsc::unbounded_channel();
            match tokio::runtime::Handle::try_current() {
                Ok(rt) => drop(rt.spawn(serve(self.inner.backend.clone(), rx, tx.downgrade()))),
                Err(_) => tracing::warn!("notifications: no runtime to show them"),
            }
            tx
        });
        let _ = queue.send(cmd);
    }

    /// Show `note` with the settings' `sound` (None: silent), replacing its conversation's bubble; `on_click` runs when it
    /// is clicked (on freedesktop desktops; elsewhere a click does what the system does).
    pub fn show(&self, note: Note, sound: Option<String>, on_click: Box<dyn FnOnce() + Send>) {
        let at = note.target.ts.clone().map(|ts| Place { thread: note.target.thread_ts.clone(), ts });
        let bubble = Bubble { title: note.title, body: note.body, sound, replaces: None, clickable: true };
        self.send(Cmd::Show { key: Some(note.key), at, bubble, on_click: Some(on_click) });
    }

    /// Close `key`'s bubble, if one is up, and end its waiter.
    #[allow(dead_code)] // the reads withdraw through `read`; this one for the routes after this
    pub fn withdraw(&self, key: &Key) {
        self.send(Cmd::Withdraw { key: key.clone(), read: None });
    }

    /// `key`'s conversation (`thread` None) or one of its threads was read up to message `up_to`, here or on another
    /// client: its bubble is withdrawn when it shows a message there no newer than that (a huddle's start shows none).
    pub fn read(&self, key: &Key, thread: Option<&str>, up_to: &str) {
        self.send(Cmd::Withdraw { key: key.clone(), read: Some(Place { thread: thread.map(String::from), ts: up_to.to_string() }) });
    }

    /// Show a bubble without any sound, ttyak's or the desktop's, that nothing replaces (the update notice: quiet hours
    /// do not hold it back, so it must never sound); `on_click` as in [`Notifier::show`].
    pub fn show_silent(&self, title: String, body: String, on_click: Option<Box<dyn FnOnce() + Send>>) {
        let bubble = Bubble { title, body, sound: None, replaces: None, clickable: on_click.is_some() };
        self.send(Cmd::Show { key: None, at: None, bubble, on_click });
    }

    /// How bubbles, their sound and clicks go on this machine.
    #[allow(dead_code)] // read by F6, wired in after this
    pub fn route(&self) -> RouteInfo {
        let click = if cfg!(all(unix, not(target_os = "macos"))) { "opens it in ttyak" } else { "does what the system does" };
        RouteInfo { shown_by: "the system's notifications".into(), sound_by: "ttyak".into(), click: click.into() }
    }

    /// Ask for the bell (the run loop writes it between draws).
    pub fn ring(&self) {
        self.inner.bell.store(true, Ordering::Relaxed);
    }

    /// Whether a bell is due, once.
    pub fn take_bell(&self) -> bool {
        self.inner.bell.swap(false, Ordering::Relaxed)
    }

    /// The terminal reported its focus.
    pub fn set_focus(&self, focus: TermFocus) {
        self.inner.focus.store(focus as u8, Ordering::Relaxed);
    }

    /// The terminal's focus as last reported.
    pub fn focus(&self) -> TermFocus {
        match self.inner.focus.load(Ordering::Relaxed) {
            f if f == TermFocus::Focused as u8 => TermFocus::Focused,
            f if f == TermFocus::Unfocused as u8 => TermFocus::Unfocused,
            _ => TermFocus::Unknown,
        }
    }
}

/// The worker: shows, replaces and closes bubbles in the order asked, on `backend`.
async fn serve(backend: Arc<dyn Backend>, mut queue: mpsc::UnboundedReceiver<Cmd>, back: mpsc::WeakUnboundedSender<Cmd>) {
    let mut live: HashMap<Key, Live> = HashMap::new();
    let mut generation = 0;
    while let Some(cmd) = queue.recv().await {
        match cmd {
            Cmd::Show { key, at, mut bubble, on_click } => {
                let old = key.as_ref().and_then(|k| live.remove(k));
                // The replaced bubble's waiter ends: the server keeps its id for the new one.
                if let Some(w) = old.as_ref().and_then(|o| o.waiter.as_ref()) {
                    w.abort();
                }
                bubble.replaces = old.map(|o| o.shown.id());
                let shown = match backend.show(bubble).await {
                    Ok(shown) => shown,
                    Err(e) => {
                        tracing::warn!("notification failed: {e}");
                        continue;
                    }
                };
                generation += 1;
                let ended = key.clone().map(|k| (back.clone(), k, generation));
                let waiter = on_click.map(|f| tokio::spawn(wait(shown.clone(), f, ended)).abort_handle());
                if let Some(key) = key {
                    live.insert(key, Live { shown, at, generation, waiter });
                }
            }
            Cmd::Withdraw { key, read } => {
                let covered = live.get(&key).is_some_and(|l| match &read {
                    None => true,
                    Some(read) => l.at.as_ref().is_some_and(|at| at.read_by(read)),
                });
                if let Some(l) = covered.then(|| live.remove(&key)).flatten() {
                    if let Some(w) = l.waiter {
                        w.abort();
                    }
                    l.shown.close().await;
                    tracing::debug!(workspace = key.workspace + 1, "notification withdrawn");
                }
            }
            Cmd::Ended { key, generation, gone } => {
                if let Some(l) = live.get_mut(&key).filter(|l| l.generation == generation) {
                    l.waiter = None;
                    if gone {
                        live.remove(&key);
                    }
                }
            }
            #[cfg(test)]
            Cmd::Settled(done) => drop(done.send(())),
        }
    }
}

/// Wait for a click on `shown` (at most [`CLICK_WAIT`]) and run `on_click` for it; then tell the worker (`ended`).
async fn wait(shown: Arc<dyn Shown>, on_click: Box<dyn FnOnce() + Send>, ended: Option<(mpsc::WeakUnboundedSender<Cmd>, Key, u64)>) {
    let gone = match tokio::time::timeout(CLICK_WAIT, shown.clicked()).await {
        Ok(true) => {
            on_click();
            true
        }
        Ok(false) => true,
        // Maybe still in the notification centre: kept for replacing and withdrawing.
        Err(_) => false,
    };
    if let Some((back, key, generation)) = ended {
        if let Some(back) = back.upgrade() {
            let _ = back.send(Cmd::Ended { key, generation, gone });
        }
    }
}

/// ttyak's own notification sound, as the settings say (no route plays it yet).
fn play_sound(settings: &super::settings::Settings) {
    super::sound::play(&settings.notifications.sound, super::settings::Settings::device(huddle::devices::OUTPUT_ENV, &settings.audio.output), settings.audio.sound_volume);
}

/// Fire what `out` lets through for `note`: the bubble (replacing its conversation's) with its sound, and the bell.
pub fn interrupt(notifier: &Notifier, note: Note, out: &Outputs, settings: &super::settings::Settings, on_click: Box<dyn FnOnce() + Send>) {
    if out.bubble {
        if out.sound {
            play_sound(settings);
        }
        notifier.show(note, out.sound.then(|| settings.notifications.sound.clone()), on_click);
    }
    if out.bell {
        notifier.ring();
    }
}

/// The terminal's bell: terminals turn it into an urgency hint, tmux flags the window.
pub fn bell(out: &mut impl Write) -> io::Result<()> {
    out.write_all(b"\x07")?;
    out.flush()
}

/// A notification server in memory, for tests.
#[cfg(test)]
pub mod fake {
    use super::*;
    use std::sync::Mutex;
    use std::sync::atomic::AtomicU32;

    /// What the server was asked.
    #[derive(Debug, Clone, PartialEq)]
    pub enum Call {
        Show { replaces: Option<u32>, title: String, body: String, sound: Option<String>, clickable: bool },
        Close(u32),
    }

    /// Records what it is asked; clicks on demand; can be down.
    #[derive(Default)]
    pub struct Server {
        calls: Mutex<Vec<Call>>,
        last_id: AtomicU32,
        down: AtomicBool,
        clicks: Mutex<HashMap<u32, Arc<tokio::sync::Notify>>>,
    }

    impl Server {
        pub fn calls(&self) -> Vec<Call> {
            self.calls.lock().unwrap().clone()
        }

        /// From now on every bubble fails.
        pub fn go_down(&self) {
            self.down.store(true, Ordering::Relaxed);
        }

        /// The user clicks bubble `id` (the server then takes it away).
        pub fn click(&self, id: u32) {
            self.knock(id).notify_one();
        }

        fn knock(&self, id: u32) -> Arc<tokio::sync::Notify> {
            self.clicks.lock().unwrap().entry(id).or_default().clone()
        }
    }

    struct Memory(Arc<Server>);

    impl Backend for Memory {
        fn show(&self, b: Bubble) -> Fut<'static, anyhow::Result<Arc<dyn Shown>>> {
            let server = self.0.clone();
            Box::pin(async move {
                server.calls.lock().unwrap().push(Call::Show { replaces: b.replaces, title: b.title, body: b.body, sound: b.sound, clickable: b.clickable });
                if server.down.load(Ordering::Relaxed) {
                    anyhow::bail!("no notification server");
                }
                let id = b.replaces.unwrap_or_else(|| server.last_id.fetch_add(1, Ordering::Relaxed) + 1);
                Ok(Arc::new(OnScreen { id, server }) as Arc<dyn Shown>)
            })
        }
    }

    struct OnScreen {
        id: u32,
        server: Arc<Server>,
    }

    impl Shown for OnScreen {
        fn id(&self) -> u32 {
            self.id
        }

        fn clicked(&self) -> Fut<'_, bool> {
            let knock = self.server.knock(self.id);
            Box::pin(async move {
                knock.notified().await;
                true
            })
        }

        fn close(&self) -> Fut<'_, ()> {
            self.server.calls.lock().unwrap().push(Call::Close(self.id));
            Box::pin(async {})
        }
    }

    impl Notifier {
        /// A notifier on a server in memory.
        pub fn fake() -> (Notifier, Arc<Server>) {
            let server = Arc::new(Server::default());
            (Notifier::with(Arc::new(Memory(server.clone()))), server)
        }

        /// Once everything asked so far is done.
        pub async fn settled(&self) {
            let (done, settled) = tokio::sync::oneshot::channel();
            self.send(Cmd::Settled(done));
            let _ = tokio::time::timeout(std::time::Duration::from_secs(5), settled).await;
        }

        /// Once `done` holds (a click went through) and everything asked so far is done.
        pub async fn settled_after(&self, done: impl Fn() -> bool) {
            let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(5);
            while !done() && tokio::time::Instant::now() < deadline {
                tokio::time::sleep(std::time::Duration::from_millis(5)).await;
            }
            self.settled().await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use slack_core::types::{Conversation, User};
    use std::sync::Mutex;

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
    fn dms_and_mentions_notify() {
        let w = ws();
        assert_eq!(notification(&w, "D1", &msg("U2", "hi"), false, 0.0), Some(("Bob".into(), "hi".into())));
        assert_eq!(notification(&w, "C1", &msg("U2", "hey <@UME>"), false, 0.0).map(|n| n.0), Some("Bob in #general".into()));
        assert_eq!(notification(&w, "C1", &msg("U2", "<!here> lunch"), false, 0.0).map(|n| n.1), Some("@here lunch".into()));
        assert!(notification(&w, "C1", &msg("U2", "plain chatter"), false, 0.0).is_none());
    }

    #[test]
    fn click_targets() {
        let top = msg("U2", "hi");
        assert_eq!(Target::message("C1", &top), Target { channel: "C1".into(), ts: Some("10.0".into()), thread_ts: None });
        let mut parent = msg("U2", "a thread");
        parent.thread_ts = Some("10.0".into());
        assert_eq!(Target::message("C1", &parent).thread_ts, None, "a thread's parent opens in the channel");
        let mut reply = msg("U2", "a reply");
        (reply.ts, reply.thread_ts) = ("12.0".into(), Some("10.0".into()));
        assert_eq!(Target::message("C1", &reply), Target { channel: "C1".into(), ts: Some("12.0".into()), thread_ts: Some("10.0".into()) });
    }

    #[test]
    fn quiet_cases() {
        let mut w = ws();
        assert!(notification(&w, "D1", &msg("UME", "mine"), false, 0.0).is_none());
        assert!(notification(&w, "D1", &msg("U2", "seen"), true, 0.0).is_none());
        let mut join = msg("U2", "joined");
        join.subtype = Some("channel_join".into());
        assert!(notification(&w, "D1", &join, false, 0.0).is_none());
        w.dnd.snooze_until = Some(100.0);
        assert!(notification(&w, "D1", &msg("U2", "dnd"), false, 50.0).is_none());
        assert!(notification(&w, "D1", &msg("U2", "after dnd"), false, 150.0).is_some());
        w.muted.insert("C1".into());
        assert!(notification(&w, "C1", &msg("U2", "<@UME>"), false, 150.0).is_none());
    }

    #[test]
    fn levels_and_keywords() {
        let mut w = ws();
        w.notify.keywords = vec!["deploy".into()];
        assert!(notification(&w, "C1", &msg("U2", "we deploy at 5"), false, 0.0).is_some(), "a keyword counts as a mention");
        w.notify.channels.insert("C1".into(), "everything".into());
        assert!(notification(&w, "C1", &msg("U2", "plain chatter"), false, 0.0).is_some());
        let mut reply = msg("U2", "a reply elsewhere");
        reply.thread_ts = Some("1.0".into());
        assert!(notification(&w, "C1", &reply, false, 0.0).is_none(), "everything: not every reply");
        w.notify.channels.insert("C1".into(), "nothing".into());
        assert!(notification(&w, "C1", &msg("U2", "hey <@UME>"), false, 0.0).is_none());
        w.notify.global = "nothing".into();
        assert!(notification(&w, "D1", &msg("U2", "hi"), false, 0.0).is_none(), "nothing silences DMs too");
    }

    #[test]
    fn the_update_notice() {
        let latest = crate::update::Latest { version: semver::Version::new(9, 9, 9), date: String::new(), asset: crate::update::Asset { url: String::new(), sha256: String::new() } };
        assert_eq!(update_notification(&latest, "0.10.0"), ("ttyak 9.9.9 is out".into(), "You have 0.10.0. See F6 → Updates.".into()));
    }

    #[test]
    fn the_update_notice_is_silent() {
        let quiet = notice("t", "b", None);
        let loud = notice("t", "b", Some("system"));
        assert_eq!((quiet.summary.as_str(), quiet.body.as_str(), quiet.appname.as_str()), ("t", "b", "ttyak"));
        #[cfg(all(unix, not(target_os = "macos")))]
        {
            use notify_rust::Hint;
            assert!(quiet.hints.contains(&Hint::SuppressSound(true)) && !quiet.hints.iter().any(|h| matches!(h, Hint::SoundName(_))), "{:?}", quiet.hints);
            assert!(!loud.hints.contains(&Hint::SuppressSound(true)) && loud.hints.iter().any(|h| matches!(h, Hint::SoundName(_))), "{:?}", loud.hints);
        }
        let _ = loud;
    }

    #[test]
    fn huddle_starts_in_dms_only() {
        let w = ws();
        assert_eq!(huddle_notification(&w, "D1", "U2", false, 0.0, "F3").map(|n| n.0), Some("Bob started a huddle".into()));
        assert!(huddle_notification(&w, "C1", "U2", false, 0.0, "F3").is_none());
        assert!(huddle_notification(&w, "D1", "U2", true, 0.0, "F3").is_none());
        assert!(huddle_notification(&w, "D1", "UME", false, 0.0, "F3").is_none());
    }

    fn note(channel: &str, ts: &str, thread: Option<&str>, text: &str) -> Note {
        let target = Target { channel: channel.into(), ts: Some(ts.into()), thread_ts: thread.map(String::from) };
        Note { key: Key { workspace: 0, channel: channel.into() }, title: "Bob".into(), body: text.into(), target }
    }

    fn shown(replaces: Option<u32>, body: &str) -> fake::Call {
        fake::Call::Show { replaces, title: "Bob".into(), body: body.into(), sound: None, clickable: true }
    }

    /// What each click ran, by the body of the note it was for.
    fn recorder() -> (Arc<Mutex<Vec<&'static str>>>, impl Fn(&'static str) -> Box<dyn FnOnce() + Send>) {
        let ran = Arc::new(Mutex::new(Vec::new()));
        let r = ran.clone();
        (ran, move |what| {
            let r = r.clone();
            Box::new(move || r.lock().unwrap().push(what)) as Box<dyn FnOnce() + Send>
        })
    }

    #[tokio::test]
    async fn a_newer_message_replaces_the_bubble() {
        let (n, server) = Notifier::fake();
        let (clicked, on_click) = recorder();
        n.show(note("D1", "1.0", None, "one"), None, on_click("one"));
        n.show(note("D1", "2.0", None, "two"), None, on_click("two"));
        n.show(note("D2", "3.0", None, "elsewhere"), None, on_click("elsewhere"));
        n.settled().await;
        assert_eq!(server.calls(), [shown(None, "one"), shown(Some(1), "two"), shown(None, "elsewhere")], "one bubble per conversation");
        server.click(1);
        n.settled_after(|| !clicked.lock().unwrap().is_empty()).await;
        assert_eq!(*clicked.lock().unwrap(), ["two"], "the replaced bubble's waiter has ended");
        // Clicked, it is gone: the next one is a bubble of its own.
        n.show(note("D1", "4.0", None, "three"), Some("system".into()), on_click("three"));
        n.settled().await;
        let fake::Call::Show { replaces, sound, .. } = server.calls().pop().unwrap() else { panic!() };
        assert_eq!((replaces, sound.as_deref()), (None, Some("system")));
        // The update notice stands alone, silent.
        n.show_silent("ttyak 9.9.9 is out".into(), "b".into(), None);
        n.show_silent("ttyak 9.9.9 is out".into(), "b".into(), None);
        n.settled().await;
        let calls = server.calls();
        assert!(calls[calls.len() - 2..].iter().all(|c| matches!(c, fake::Call::Show { replaces: None, sound: None, clickable: false, .. })), "{calls:?}");
    }

    #[tokio::test]
    async fn reading_withdraws_it() {
        let (n, server) = Notifier::fake();
        let key = Key { workspace: 0, channel: "D1".into() };
        n.show(note("D1", "1.0", None, "one"), None, Box::new(|| {}));
        n.withdraw(&key);
        n.withdraw(&key);
        n.settled().await;
        assert_eq!(server.calls(), [shown(None, "one"), fake::Call::Close(1)], "closed once");
        // Only a read that covers what it shows withdraws it.
        n.show(note("D1", "5.0", None, "five"), None, Box::new(|| {}));
        n.read(&key, None, "4.0");
        n.read(&key, Some("5.0"), "9.0");
        n.read(&Key { workspace: 1, channel: "D1".into() }, None, "9.0");
        n.settled().await;
        assert_eq!(server.calls().len(), 3, "an older mark, a thread, another workspace: {:?}", server.calls());
        n.read(&key, None, "5.0");
        n.settled().await;
        assert_eq!(server.calls().pop(), Some(fake::Call::Close(2)));
        // A reply's bubble goes with its thread, not with the conversation.
        n.show(note("D1", "6.0", Some("3.0"), "a reply"), None, Box::new(|| {}));
        n.read(&key, None, "9.0");
        n.settled().await;
        assert_eq!(server.calls().len(), 5);
        n.read(&key, Some("3.0"), "6.0");
        n.settled().await;
        assert_eq!(server.calls().pop(), Some(fake::Call::Close(3)));
    }

    #[tokio::test]
    async fn no_server_no_panic() {
        let (n, server) = Notifier::fake();
        server.go_down();
        n.show(note("D1", "1.0", None, "one"), None, Box::new(|| {}));
        n.withdraw(&Key { workspace: 0, channel: "D1".into() });
        n.ring();
        n.settled().await;
        assert_eq!(server.calls(), [shown(None, "one")], "asked once, nothing to close");
        let mut out = Vec::new();
        assert!(n.take_bell());
        bell(&mut out).unwrap();
        assert_eq!(out, b"\x07", "the bell still rings");
        assert!(!n.take_bell(), "once");
    }

    #[test]
    fn replies_in_my_threads() {
        let mut w = ws();
        w.timelines.entry("C1".into()).or_default().upsert(Message { ts: "5.0".into(), user: Some("UME".into()), ..Default::default() });
        let mut reply = msg("U2", "answer");
        reply.thread_ts = Some("5.0".into());
        assert_eq!(notification(&w, "C1", &reply, false, 0.0).map(|n| n.0), Some("Bob in #general (thread)".into()));
        reply.thread_ts = Some("6.0".into());
        assert!(notification(&w, "C1", &reply, false, 0.0).is_none());
    }
}
