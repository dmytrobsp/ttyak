//! Desktop notifications for incoming messages: one bubble per conversation, withdrawn once read, and the bell.

use std::collections::HashMap;
use std::future::Future;
use std::io::{self, Write};
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicU8, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use slack_core::types::Message;
use tokio::sync::mpsc;

use super::attention::{self, Decision, Key, Note, Outputs, TermFocus};
use super::osc99;
use super::state::Workspace;
use super::system_dnd::{self, Environment, Server, SoundBy};

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

/// Whose bubble was clicked: the terminal's own (it brought its window forward already) or the system notifier's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClickedOn {
    Terminal,
    System,
}

/// What runs when a bubble is clicked, told whose bubble it was.
pub type OnClick = Box<dyn FnOnce(ClickedOn) + Send>;

impl Target {
    pub fn message(channel: &str, m: &Message) -> Target {
        Target { channel: channel.to_string(), ts: Some(m.ts.clone()), thread_ts: m.thread_ts.clone().filter(|_| m.is_reply()) }
    }
}

/// A notification titled `title` saying `body`, with the settings' `sound` (None: none at all) as `by` carries it:
/// `Notification`: the server plays it ("system": the desktop's own, else the built-in one as the WAV `file`); otherwise
/// the server is told to keep quiet (ttyak plays it itself, or nobody does). (macOS and Windows send their bubbles
/// themselves: [`mac`], [`toast`].)
#[cfg_attr(not(all(unix, not(target_os = "macos"))), allow(dead_code))]
fn notice(title: &str, body: &str, sound: Option<&str>, by: SoundBy, file: Option<&std::path::Path>) -> notify_rust::Notification {
    let mut n = notify_rust::Notification::new();
    n.appname("ttyak").summary(title).body(body);
    let by_server = sound.filter(|_| by == SoundBy::Notification);
    match by_server {
        Some("system") => {
            n.sound_name(super::sound::system_sound_name());
        }
        // (Without a sound name macOS and Windows stay silent; XDG servers may play their own.)
        #[cfg(all(unix, not(target_os = "macos")))]
        Some(_) => match file {
            Some(file) => {
                n.hint(notify_rust::Hint::SoundFile(file.to_string_lossy().into_owned()));
            }
            None => {
                n.hint(notify_rust::Hint::SuppressSound(true));
            }
        },
        #[cfg(all(unix, not(target_os = "macos")))]
        None => {
            n.hint(notify_rust::Hint::SuppressSound(true));
        }
        #[cfg(not(all(unix, not(target_os = "macos"))))]
        Some(_) | None => {
            let _ = file;
        }
    }
    n
}

/// How long a click on a notification still counts (the notification centre keeps them).
const CLICK_WAIT: std::time::Duration = std::time::Duration::from_secs(30 * 60);

/// How long the worker waits for the server to show or close a bubble before it goes on with the next (an internal
/// wait: a hung server must not hold back every notification after it).
const SERVER_WAIT: std::time::Duration = std::time::Duration::from_millis(if cfg!(test) { 300 } else { 5000 });

/// A future a notification backend hands back.
type Fut<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// What a bubble shows.
#[derive(Debug, Clone)]
struct Bubble {
    title: String,
    body: String,
    /// The settings' sound ("system": the desktop's); None: silent.
    sound: Option<String>,
    /// Who plays it (the worker decides from what the server can do).
    by: SoundBy,
    /// The WAV file of the built-in sound, when the server plays it.
    file: Option<std::path::PathBuf>,
    /// The server's id of the bubble it replaces.
    replaces: Option<u32>,
    /// Its conversation's id ([`osc99::id`]; None: one no other replaces): Windows replaces and withdraws a toast by it.
    #[cfg_attr(not(windows), allow(dead_code))]
    tag: Option<String>,
    /// A click is waited for.
    clickable: bool,
}

/// Where bubbles are shown: the system's notifier (tests use one in memory).
trait Backend: Send + Sync {
    fn show(&self, bubble: Bubble) -> Fut<'static, anyhow::Result<Arc<dyn Shown>>>;
    /// What the notification server is and can do.
    fn environment(&self) -> Fut<'static, anyhow::Result<Environment>>;
    /// Whether the system holds notifications back right now (None: cannot be told).
    fn suppressed(&self, server: Server) -> Fut<'static, Option<bool>>;
    /// Play `sound` in ttyak's own player.
    fn play(&self, sound: Sound);
    /// The built-in sound `name` as a WAV file, for a server that plays files (None: it cannot be written).
    fn sound_file(&self, name: &str) -> Option<std::path::PathBuf>;
}

/// A sound ttyak plays itself: the settings' name, the output device (None: the default) and the volume in percent.
#[derive(Debug, Clone, PartialEq)]
pub struct Sound {
    pub name: String,
    pub device: Option<String>,
    pub volume: u32,
}

impl Sound {
    /// The notification sound the settings pick, on the speaker and at the volume they pick.
    pub fn of(settings: &super::settings::Settings) -> Sound {
        let device = super::settings::Settings::device(huddle::devices::OUTPUT_ENV, &settings.audio.output);
        Sound { name: settings.notifications.sound.clone(), device, volume: settings.audio.sound_volume }
    }
}

/// What holds back an interruption in the worker, asked there so the run loop never waits for the system.
#[derive(Debug, Clone, Copy, Default)]
pub struct Guard {
    /// Ask the system whether notifications are suppressed first; if so, nothing happens at all.
    pub dnd: bool,
    /// Ring the bell (after that question).
    pub bell: bool,
    /// The focus period the event was decided in (set by the notifier): the bell is for that one only.
    epoch: u64,
}

/// A bubble on screen.
trait Shown: Send + Sync {
    /// The server's id.
    fn id(&self) -> u32;
    /// Until it is clicked (true) or closed (false).
    fn clicked(&self) -> Fut<'_, bool>;
    /// Take it off the screen.
    fn close(&self) -> Fut<'_, ()>;
    /// A newer bubble can't take its place on screen: it is closed before the newer one is shown.
    fn closed_to_replace(&self) -> bool {
        false
    }
}

/// The system's notifier: freedesktop's notification server through notify-rust (replacing, closing, clicks), macOS's
/// notification centre ([`mac`]: clicks while the main thread runs its run loop), or Windows' toasts ([`toast`]: replaced
/// and withdrawn by their tag, clicks on screen).
struct System;

impl Backend for System {
    fn show(&self, bubble: Bubble) -> Fut<'static, anyhow::Result<Arc<dyn Shown>>> {
        Box::pin(async move {
            #[cfg(all(unix, not(target_os = "macos")))]
            {
                let mut n = notice(&bubble.title, &bubble.body, bubble.sound.as_deref(), bubble.by, bubble.file.as_deref());
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
            #[cfg(target_os = "macos")]
            {
                // (The older API replaces nothing in place: the worker takes the older bubble off first.)
                let (title, body, sound) = (bubble.title, bubble.body, mac_sound(bubble.sound.as_deref(), bubble.by));
                let shown = MacBubble { title: title.clone(), body: body.clone(), answer: Mutex::new(None) };
                if bubble.clickable && mac::may_wait() {
                    let (tx, rx) = tokio::sync::oneshot::channel();
                    // A thread of its own, not the runtime's blocking pool: it waits until the bubble is clicked or leaves
                    // the notification centre (taken off by ttyak, or by the user).
                    let waiter = std::thread::Builder::new().name("notification".into()).spawn(move || {
                        let _ = tx.send(mac::send(&title, &body, sound, true));
                        mac::done_waiting();
                    });
                    if let Err(e) = waiter {
                        mac::done_waiting();
                        return Err(e.into());
                    }
                    *shown.answer.lock().unwrap_or_else(|e| e.into_inner()) = Some(rx);
                } else {
                    tokio::task::spawn_blocking(move || mac::send(&title, &body, sound, false));
                }
                Ok(Arc::new(shown) as Arc<dyn Shown>)
            }
            #[cfg(windows)]
            {
                // (Replaced in place by its tag: a newer toast with it takes the older one's place.)
                let sound = bubble.sound.as_deref() == Some("system") && bubble.by == SoundBy::Notification;
                let tag = bubble.tag.clone();
                let shown = tokio::task::spawn_blocking(move || toast::show(&bubble.title, &bubble.body, sound, bubble.tag.as_deref(), &toast::own_group())).await??;
                tracing::debug!(?tag, "notification shown by Windows");
                Ok(Arc::new(shown) as Arc<dyn Shown>)
            }
        })
    }

    fn environment(&self) -> Fut<'static, anyhow::Result<Environment>> {
        Box::pin(system_dnd::discover())
    }

    fn suppressed(&self, server: Server) -> Fut<'static, Option<bool>> {
        Box::pin(async move { system_dnd::suppressed(&server).await })
    }

    fn play(&self, sound: Sound) {
        super::sound::play(&sound.name, sound.device, sound.volume);
    }

    fn sound_file(&self, name: &str) -> Option<std::path::PathBuf> {
        #[cfg(all(unix, not(target_os = "macos")))]
        return super::sound::wav_file(name);
        #[cfg(not(all(unix, not(target_os = "macos"))))]
        return {
            let _ = name;
            None
        };
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

/// A bubble of macOS's notification centre, known by what it shows (the older API gives it no name ttyak can choose).
#[cfg(target_os = "macos")]
struct MacBubble {
    title: String,
    body: String,
    /// Its click, when one is waited for (on its own thread).
    answer: Mutex<Option<tokio::sync::oneshot::Receiver<bool>>>,
}

#[cfg(target_os = "macos")]
impl Shown for MacBubble {
    fn id(&self) -> u32 {
        0
    }

    fn clicked(&self) -> Fut<'_, bool> {
        let answer = self.answer.lock().unwrap_or_else(|e| e.into_inner()).take();
        Box::pin(async move {
            match answer {
                Some(answer) => answer.await.unwrap_or(false),
                // Not waited for: kept for replacing and withdrawing until the waiter gives up.
                None => std::future::pending().await,
            }
        })
    }

    /// Out of the notification centre (which ends its click's thread).
    fn close(&self) -> Fut<'_, ()> {
        let (title, body) = (self.title.clone(), self.body.clone());
        Box::pin(async move {
            let removed = tokio::task::spawn_blocking(move || mac::remove(&title, &body)).await.unwrap_or(0);
            tracing::debug!(removed, "notification taken out of the notification centre");
        })
    }

    fn closed_to_replace(&self) -> bool {
        true
    }
}

/// The sound macOS's bubble carries: the desktop's own when it plays "system" (any other is ttyak's own:
/// [`system_dnd::sound_route`]).
#[cfg(any(target_os = "macos", test))]
fn mac_sound(sound: Option<&str>, by: SoundBy) -> Option<&'static str> {
    (sound == Some("system") && by == SoundBy::Notification).then(super::sound::system_sound_name)
}

/// Takes one of `max` places counted by `count`; false when all are taken.
#[cfg(any(target_os = "macos", test))]
fn take_place(count: &std::sync::atomic::AtomicUsize, max: usize) -> bool {
    count.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| (n < max).then_some(n + 1)).is_ok()
}

/// macOS's notification centre through its older API, the one a program outside an app bundle can use: bubbles are sent
/// as the terminal's app (macOS 26 refuses them as apps that use the newer API, kitty's and Finder's among them), and
/// its answers — that a bubble was shown, clicked, dismissed — come through the main thread's run loop only.
#[cfg(target_os = "macos")]
pub mod mac {
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::time::Duration;

    /// How many bubbles a click is waited for at once (internal: each holds a thread until it is clicked or leaves the
    /// notification centre; any more are shown without).
    const WAITERS: usize = 32;

    /// How long the main thread's run loop runs before it looks whether ttyak ended (internal).
    const RUN_SLICE: Duration = Duration::from_millis(250);

    /// See [`listening`].
    static RUN_LOOP: AtomicBool = AtomicBool::new(false);
    static WAITING: AtomicUsize = AtomicUsize::new(0);
    static SENDER: std::sync::OnceLock<&'static str> = std::sync::OnceLock::new();

    #[link(name = "CoreFoundation", kind = "framework")]
    unsafe extern "C" {
        static kCFRunLoopDefaultMode: *const std::ffi::c_void;
        fn CFRunLoopRunInMode(mode: *const std::ffi::c_void, seconds: f64, return_after_source_handled: u8) -> i32;
        fn CFRunLoopGetMain() -> *mut std::ffi::c_void;
        fn CFRunLoopStop(run_loop: *mut std::ffi::c_void);
    }

    /// `kCFRunLoopRunFinished`: the run loop has nothing to run yet.
    const FINISHED: i32 = 1;

    /// Run `main` on a thread of its own (with the main thread's stack size) while this, the main thread, runs its run
    /// loop for the notification centre; `main`'s result once it ends (a panic in it goes on here).
    pub fn with_run_loop<T: Send + 'static>(main: impl FnOnce() -> T + Send + 'static) -> std::io::Result<T> {
        /// Wakes the main thread when `main` ends, returned or panicked.
        struct Wake(std::thread::Thread);
        impl Drop for Wake {
            fn drop(&mut self) {
                self.0.unpark();
                // SAFETY: the main thread's run loop exists for the whole process; stopping it is allowed from any
                // thread.
                unsafe { CFRunLoopStop(CFRunLoopGetMain()) };
            }
        }
        let wake = Wake(std::thread::current());
        RUN_LOOP.store(true, Ordering::Relaxed);
        let worker = std::thread::Builder::new().name("ttyak".into()).stack_size(8 << 20).spawn(move || {
            let _wake = wake;
            main()
        });
        let worker = match worker {
            Ok(worker) => worker,
            Err(e) => {
                RUN_LOOP.store(false, Ordering::Relaxed);
                return Err(e);
            }
        };
        // (Each slice also ends by itself: a wake that came between the look and the slice costs one slice.)
        while !worker.is_finished() {
            // SAFETY: CoreFoundation's exported default mode, on this thread's run loop.
            if unsafe { CFRunLoopRunInMode(kCFRunLoopDefaultMode, RUN_SLICE.as_secs_f64(), 0) } == FINISHED {
                std::thread::park_timeout(RUN_SLICE);
            }
        }
        RUN_LOOP.store(false, Ordering::Relaxed);
        match worker.join() {
            Ok(out) => Ok(out),
            Err(panic) => std::panic::resume_unwind(panic),
        }
    }

    /// Whether a click can be waited for now (the main thread runs its run loop, and a place is free); if so, the place
    /// is taken until [`done_waiting`].
    pub fn may_wait() -> bool {
        if !listening() {
            return false;
        }
        let free = super::take_place(&WAITING, WAITERS);
        if !free {
            tracing::info!("notifications: {WAITERS} wait for a click already; this one is shown without (clear the notification centre)");
        }
        free
    }

    /// The main thread runs its run loop: the notification centre's answers arrive.
    pub fn listening() -> bool {
        RUN_LOOP.load(Ordering::Relaxed)
    }

    pub fn done_waiting() {
        WAITING.fetch_sub(1, Ordering::Relaxed);
    }

    /// Who bubbles go out as when the terminal's app isn't known or installed: Finder, as mac-notification-sys would
    /// pick (said here, so its AppleScript lookup never runs; macOS 26 refuses it).
    const FALLBACK: &str = "com.apple.Finder";

    /// The app bubbles go out as: the terminal's (its icon; a click brings it forward), else [`FALLBACK`]. Decided
    /// once, before the first bubble (macOS allows it once per process).
    pub fn sender() -> &'static str {
        SENDER.get_or_init(|| {
            let vars = std::env::vars().collect();
            let host = super::super::raise::host_bundle(&vars);
            let bundle = match host {
                Some(bundle) if super::super::raise::app_installed(bundle) => bundle,
                Some(bundle) => {
                    tracing::debug!(bundle, "notifications: the terminal's app isn't installed");
                    FALLBACK
                }
                None => FALLBACK,
            };
            // (Only a bundle Launch Services knows is taken; any other would leave the swizzled default, Terminal.)
            match mac_notification_sys::set_application(bundle) {
                Ok(()) => tracing::debug!(bundle, terminal = host.is_some(), "notifications sent as this app"),
                Err(e) => tracing::warn!(bundle, "notifications: macOS would not send them as this app: {e}"),
            }
            bundle
        })
    }

    /// Take every bubble of this process titled `title` saying `body` out of the notification centre (the older API names
    /// them only by what they show); a click waited for on one then ends. How many were taken out.
    pub fn remove(title: &str, body: &str) -> usize {
        use objc2::msg_send;
        use objc2::runtime::{AnyClass, AnyObject};
        // (The bundle swizzle comes first: without it the centre is none.)
        let _ = sender();
        /// The text of NSString `s`.
        fn text(s: *mut AnyObject) -> Option<String> {
            if s.is_null() {
                return None;
            }
            // SAFETY: `s` is an NSString; its UTF-8 lives as long as the autorelease pool around this.
            let p: *const std::ffi::c_char = unsafe { msg_send![s, UTF8String] };
            // SAFETY: NUL-terminated when not null.
            (!p.is_null()).then(|| unsafe { std::ffi::CStr::from_ptr(p) }.to_string_lossy().into_owned())
        }
        objc2::rc::autoreleasepool(|_| {
            let Some(class) = AnyClass::get(c"NSUserNotificationCenter") else { return 0 };
            // SAFETY: NSUserNotificationCenter's documented class method and its array of NSUserNotification, whose
            // `title` and `informativeText` are NSStrings; `removeDeliveredNotification:` takes one of them.
            unsafe {
                let center: *mut AnyObject = msg_send![class, defaultUserNotificationCenter];
                if center.is_null() {
                    return 0;
                }
                let delivered: *mut AnyObject = msg_send![center, deliveredNotifications];
                if delivered.is_null() {
                    return 0;
                }
                let count: usize = msg_send![delivered, count];
                let mut removed = 0;
                for i in 0..count {
                    let n: *mut AnyObject = msg_send![delivered, objectAtIndex: i];
                    let (t, b): (*mut AnyObject, *mut AnyObject) = (msg_send![n, title], msg_send![n, informativeText]);
                    if text(t).as_deref() == Some(title) && text(b).as_deref() == Some(body) {
                        let _: () = msg_send![center, removeDeliveredNotification: n];
                        removed += 1;
                    }
                }
                removed
            }
        })
    }

    /// Show a bubble with `sound` (a system sound's name); `wait`: until it is clicked (true) or gone (false).
    pub fn send(title: &str, body: &str, sound: Option<&str>, wait: bool) -> bool {
        use mac_notification_sys::{Notification, NotificationResponse};
        let _ = sender();
        let mut n = Notification::new();
        n.maybe_sound(sound);
        if wait {
            n.wait_for_click(true);
        } else {
            n.asynchronous(true);
        }
        tracing::debug!(wait, "notification handed to the notification centre");
        let started = std::time::Instant::now();
        match mac_notification_sys::send_notification(title, None, body, Some(&n)) {
            Ok(NotificationResponse::Click) => {
                tracing::debug!(after = ?started.elapsed(), "notification clicked");
                true
            }
            // (Not waiting: back once macOS confirmed it, or after mac-notification-sys's 2 s without a confirmation.)
            Ok(answer) => {
                tracing::debug!(?answer, wait, after = ?started.elapsed(), "notification ended");
                false
            }
            Err(e) => {
                tracing::warn!("notification failed: {e}");
                false
            }
        }
    }
}

/// Windows' toasts, sent with the `windows` crate itself: notify-rust gives a toast no tag, so a newer one could neither
/// replace it nor a read withdraw it. They go out as Windows PowerShell's, the app id a plain program can use without
/// registering one of its own (an installer's job: ttyak stays a plain program); a click on the toast on screen reaches
/// ttyak (one in the notification centre likely not: no COM activator is registered for it).
#[cfg(any(windows, test))]
pub mod toast {
    /// Windows PowerShell's app id (notify-rust's too).
    #[cfg_attr(not(windows), allow(dead_code))]
    pub const APP_ID: &str = "{1AC14E77-02E7-4E5D-B744-2EB1AE5198B7}\\WindowsPowerShell\\v1.0\\powershell.exe";
    /// What every ttyak toast's group starts with.
    const GROUP: &str = "ttyak";
    /// Windows' longest tag and group.
    const TAG_MAX: usize = 64;

    /// The group of profile `profile`'s toasts (a tag names a toast within its group): two ttyaks on other profiles
    /// neither replace nor withdraw each other's.
    pub fn group(profile: &str) -> String {
        let name: String = profile.chars().filter(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-')).collect();
        let group = if name.is_empty() { GROUP.to_string() } else { format!("{GROUP}-{name}") };
        group.chars().take(TAG_MAX).collect()
    }

    /// The group of this process's toasts: its profile's ([`super::use_profile`]).
    #[cfg_attr(not(windows), allow(dead_code))]
    pub fn own_group() -> String {
        group(super::PROFILE.get().map_or("", String::as_str))
    }

    /// The tag of conversation `id`'s toasts ([`super::osc99::id`]): a newer one with it replaces the older in place, a
    /// read withdraws it by it.
    pub fn tag(id: &str) -> String {
        id.chars().take(TAG_MAX).collect()
    }

    /// The toast's XML: `title` over `body`, with the desktop's own sound (`sound`) or silent.
    pub fn xml(title: &str, body: &str, sound: bool) -> String {
        let audio = if sound { "" } else { r#"<audio silent="true"/>"# };
        format!(r#"<toast><visual><binding template="ToastGeneric"><text>{}</text><text>{}</text></binding></visual>{audio}</toast>"#, escape(title), escape(body))
    }

    /// `text` as XML character data: the special characters escaped, the control characters XML forbids left out.
    fn escape(text: &str) -> String {
        let mut out = String::with_capacity(text.len());
        for c in text.chars() {
            match c {
                '<' => out.push_str("&lt;"),
                '>' => out.push_str("&gt;"),
                '&' => out.push_str("&amp;"),
                '"' => out.push_str("&quot;"),
                '\'' => out.push_str("&apos;"),
                '\t' | '\n' | '\r' => out.push(c),
                c if c.is_control() && c < ' ' => {}
                c => out.push(c),
            }
        }
        out
    }

    #[cfg(windows)]
    use windows::{Data::Xml::Dom::XmlDocument, UI::Notifications::*, core::HSTRING};

    /// The toast for `title` and `body` (see [`xml`]), under `tag` in `group` when it has one.
    #[cfg(windows)]
    pub fn build(title: &str, body: &str, sound: bool, tag: Option<&str>, group: &str) -> windows::core::Result<ToastNotification> {
        let doc = XmlDocument::new()?;
        doc.LoadXml(&HSTRING::from(xml(title, body, sound)))?;
        let toast = ToastNotification::CreateToastNotification(&doc)?;
        if let Some(id) = tag {
            toast.SetTag(&HSTRING::from(self::tag(id)))?;
            toast.SetGroup(&HSTRING::from(group))?;
        }
        Ok(toast)
    }

    /// What became of a toast.
    #[cfg(windows)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum Event {
        Clicked,
        /// Off the screen (its time is up, or the user dismissed it): maybe still in the notification centre.
        OffScreen,
        /// Taken away by ttyak.
        Gone,
    }

    /// A toast on screen or in the notification centre.
    #[cfg(windows)]
    pub struct Toast {
        toast: ToastNotification,
        tag: Option<String>,
        group: String,
        events: std::sync::Mutex<Option<tokio::sync::mpsc::UnboundedReceiver<Event>>>,
    }

    /// Show `title` and `body` (see [`build`]), heard of until it is clicked or gone.
    #[cfg(windows)]
    pub fn show(title: &str, body: &str, sound: bool, tag: Option<&str>, group: &str) -> windows::core::Result<Toast> {
        use windows::Foundation::TypedEventHandler;
        let toast = build(title, body, sound, tag, group)?;
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        let clicked = tx.clone();
        toast.Activated(&TypedEventHandler::new(move |_, _| {
            let _ = clicked.send(Event::Clicked);
            Ok(())
        }))?;
        toast.Dismissed(&TypedEventHandler::new(move |_, args: windows::core::Ref<ToastDismissedEventArgs>| {
            let reason = args.as_ref().and_then(|a| a.Reason().ok());
            tracing::debug!(?reason, "toast dismissed");
            let _ = tx.send(if reason == Some(ToastDismissalReason::ApplicationHidden) { Event::Gone } else { Event::OffScreen });
            Ok(())
        }))?;
        ToastNotificationManager::CreateToastNotifierWithId(&HSTRING::from(APP_ID))?.Show(&toast)?;
        Ok(Toast { toast, tag: tag.map(self::tag), group: group.to_string(), events: std::sync::Mutex::new(Some(rx)) })
    }

    /// Take `toast` off the screen and the toast tagged `tag` in `group` out of the notification centre.
    #[cfg(windows)]
    pub fn withdraw(toast: Option<&ToastNotification>, tag: Option<&str>, group: &str) -> windows::core::Result<()> {
        if let Some(toast) = toast {
            // (Fails for one off the screen already; the notification centre's copy goes below.)
            if let Err(e) = ToastNotificationManager::CreateToastNotifierWithId(&HSTRING::from(APP_ID)).and_then(|n| n.Hide(toast)) {
                tracing::debug!("toast not hidden: {e}");
            }
        }
        match tag {
            Some(tag) => ToastNotificationManager::History()?.RemoveGroupedTagWithId(&HSTRING::from(tag), &HSTRING::from(group), &HSTRING::from(APP_ID)),
            None => Ok(()),
        }
    }

    /// Windows' notification setting for PowerShell's app id (`NotificationSetting`: 0 enabled, else why not); None when
    /// it can't be read.
    #[cfg(windows)]
    pub fn setting() -> Option<i32> {
        ToastNotificationManager::CreateToastNotifierWithId(&HSTRING::from(APP_ID)).and_then(|n| n.Setting()).map(|s| s.0).ok()
    }

    #[cfg(windows)]
    impl super::Shown for Toast {
        /// (Replaced by its tag, not the server's id.)
        fn id(&self) -> u32 {
            0
        }

        /// A click on it on screen (in the notification centre: not tried, likely never heard); gone: not clicked.
        fn clicked(&self) -> super::Fut<'_, bool> {
            let events = self.events.lock().unwrap_or_else(|e| e.into_inner()).take();
            Box::pin(async move {
                let Some(mut events) = events else { return false };
                loop {
                    match events.recv().await {
                        Some(Event::Clicked) => return true,
                        // (Kept: a read still takes it out of the notification centre, a newer one still replaces it.)
                        Some(Event::OffScreen) => continue,
                        Some(Event::Gone) | None => return false,
                    }
                }
            })
        }

        fn close(&self) -> super::Fut<'_, ()> {
            let (toast, tag, group) = (self.toast.clone(), self.tag.clone(), self.group.clone());
            Box::pin(async move {
                let done = tokio::task::spawn_blocking(move || withdraw(Some(&toast), tag.as_deref(), &group)).await;
                match done {
                    Ok(Ok(())) => tracing::debug!("toast withdrawn"),
                    Ok(Err(e)) => tracing::debug!("toast not withdrawn: {e}"),
                    Err(e) => tracing::debug!("toast not withdrawn: {e}"),
                }
            })
        }
    }
}

/// How long after a bubble went out a close reported for its id is taken for the bubble it replaced (an internal wait:
/// the terminal's report carries only the id, and a bubble dismissed just as a newer one went out is closed for good,
/// so the newer one shows on its own).
const CLOSE_GUARD: std::time::Duration = if cfg!(test) { std::time::Duration::ZERO } else { std::time::Duration::from_secs(1) };

/// The profile this ttyak runs (Windows: its toasts' group).
static PROFILE: OnceLock<String> = OnceLock::new();

/// This ttyak runs profile `name` (said once, at start).
pub fn use_profile(name: &str) {
    let _ = PROFILE.set(name.to_string());
}

/// Bubbles through the terminal itself (OSC 99): their sequences wait for the run loop to write them between draws, and
/// the terminal's reports end their waiters (a click focuses the terminal's window and tab).
struct Terminal {
    /// What the terminal is called, for F6.
    name: String,
    /// [`CLOSE_GUARD`] (tests change it).
    guard: std::time::Duration,
    /// The sequences not written yet.
    out: Mutex<Vec<String>>,
    /// Wakes the run loop to write them.
    due: tokio::sync::Notify,
    /// The click waiter of each bubble on screen that waits for a click, by its id.
    waiting: Mutex<HashMap<String, Waiter>>,
}

/// Where the terminal's report about a bubble goes.
struct Waiter {
    /// True clicked, false closed.
    answer: mpsc::UnboundedSender<bool>,
    /// When the run loop wrote the bubble (None: not yet).
    written: Option<std::time::Instant>,
}

impl Terminal {
    fn new(name: String, guard: std::time::Duration) -> Terminal {
        Terminal { name, guard, out: Mutex::default(), due: tokio::sync::Notify::new(), waiting: Mutex::default() }
    }

    /// Hand `seq` to the run loop.
    fn write(&self, seq: String) {
        self.out.lock().unwrap_or_else(|e| e.into_inner()).push(seq);
        self.due.notify_one();
    }

    /// The sequences to write now: the bubbles among them count as written from now on. (The waiters are locked first,
    /// as in [`Terminal::show`], so a bubble is stamped only with its sequence.)
    fn take(&self) -> Vec<String> {
        let mut waiting = self.waiting.lock().unwrap_or_else(|e| e.into_inner());
        let out = std::mem::take(&mut *self.out.lock().unwrap_or_else(|e| e.into_inner()));
        if !out.is_empty() {
            let now = std::time::Instant::now();
            for w in waiting.values_mut() {
                w.written.get_or_insert(now);
            }
        }
        out
    }

    /// Show `bubble` as `id` (the same id replaces it on screen; the waiter of the one it replaces ends), waiting for a
    /// click when it is clickable. Waiters that stopped waiting (after [`CLICK_WAIT`]) are dropped.
    fn show(self: &Arc<Self>, id: String, bubble: &Bubble) -> Arc<dyn Shown> {
        let mut waiting = self.waiting.lock().unwrap_or_else(|e| e.into_inner());
        waiting.retain(|_, w| !w.answer.is_closed());
        let answers = bubble.clickable.then(|| {
            let (answer, answers) = mpsc::unbounded_channel();
            waiting.insert(id.clone(), Waiter { answer, written: None });
            answers
        });
        if answers.is_none() {
            waiting.remove(&id);
        }
        self.write(osc99::show(&id, &bubble.title, &bubble.body, Some(osc99::sound_name(bubble.by))));
        drop(waiting);
        tracing::debug!(id, "notification shown by the terminal");
        Arc::new(OnTerminal { id, terminal: self.clone(), answers: Mutex::new(answers) })
    }

    /// The terminal reported a click on a bubble, or its closing (not the closing of the one a bubble written within
    /// [`CLOSE_GUARD`] replaced: that one is still on screen).
    fn report(&self, report: osc99::Report) {
        let mut waiting = self.waiting.lock().unwrap_or_else(|e| e.into_inner());
        match report {
            osc99::Report::Clicked(id) => {
                tracing::debug!(id, "notification clicked in the terminal");
                if let Some(w) = waiting.remove(&id) {
                    let _ = w.answer.send(true);
                }
            }
            osc99::Report::Closed(id) => {
                let current = waiting.get(&id).is_some_and(|w| w.written.is_some_and(|at| at.elapsed() >= self.guard));
                if let Some(w) = current.then(|| waiting.remove(&id)).flatten() {
                    let _ = w.answer.send(false);
                } else if waiting.contains_key(&id) {
                    tracing::debug!(id, "a close for the bubble the terminal's newer one replaced");
                }
            }
        }
    }
}

/// A bubble the terminal shows.
struct OnTerminal {
    id: String,
    terminal: Arc<Terminal>,
    /// What the terminal reports about it (taken by the one waiter).
    answers: Mutex<Option<mpsc::UnboundedReceiver<bool>>>,
}

impl Shown for OnTerminal {
    /// (The terminal replaces by the bubble's own id, not the server's.)
    fn id(&self) -> u32 {
        0
    }

    fn clicked(&self) -> Fut<'_, bool> {
        let answers = self.answers.lock().unwrap_or_else(|e| e.into_inner()).take();
        Box::pin(async move {
            match answers {
                // A newer bubble with its id ends it (its sender is dropped): not clicked.
                Some(mut answers) => answers.recv().await.unwrap_or(false),
                None => false,
            }
        })
    }

    fn close(&self) -> Fut<'_, ()> {
        self.terminal.waiting.lock().unwrap_or_else(|e| e.into_inner()).remove(&self.id);
        self.terminal.write(osc99::close(&self.id));
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
    /// What the worker shares with the rest.
    shared: Arc<Shared>,
    /// The terminal's focus as last reported (the other workspaces' watchers share the window).
    focus: AtomicU8,
}

/// How long the system's answer about do-not-disturb is reused for the next event (an internal wait: a burst of
/// messages asks the system once, not once each).
const DND_REUSE: std::time::Duration = std::time::Duration::from_secs(2);

/// How long a failed look for the notification server is remembered before the next (an internal wait: a hung bus
/// costs [`SERVER_WAIT`] once, not for every command).
const DISCOVERY_RETRY: std::time::Duration = std::time::Duration::from_secs(30);

/// What the worker shares with the run loop.
struct Shared {
    /// A bell is due, as the focus period (plus one) it was decided in; 0: none.
    bell: AtomicU64,
    /// Counts the changes of the terminal's focus: a bell is for the period it was decided in only.
    epoch: AtomicU64,
    /// The notification server, once found (a failed show forgets it: the server may have changed).
    environment: Mutex<Option<Environment>>,
    /// When looking for the server last failed.
    discovery_failed: Mutex<Option<std::time::Instant>>,
    /// The system's last answer about do-not-disturb, and when it came.
    dnd: Mutex<Option<(std::time::Instant, Option<bool>)>>,
    /// The server was looked for for F6 already.
    probed: AtomicBool,
    /// [`DND_REUSE`] and [`DISCOVERY_RETRY`] (tests shorten them).
    reuse: std::time::Duration,
    retry: std::time::Duration,
    /// The terminal, once it said at start that it shows notifications itself.
    terminal: OnceLock<Arc<Terminal>>,
}

impl Shared {
    fn new(reuse: std::time::Duration, retry: std::time::Duration) -> Shared {
        Shared { bell: AtomicU64::new(0), epoch: AtomicU64::new(0), environment: Mutex::default(), discovery_failed: Mutex::default(), dnd: Mutex::default(), probed: AtomicBool::new(false), reuse, retry, terminal: OnceLock::new() }
    }
}

/// A request to the worker.
enum Cmd {
    /// Show a bubble; `key` None: one no other replaces or read withdraws (the update notice). `own`: ttyak's sound,
    /// played if the server will not play it; `guard`: what is asked first.
    Show { key: Option<Key>, at: Option<Place>, bubble: Bubble, own: Option<Sound>, guard: Guard, on_click: Option<OnClick> },
    /// Ring the bell unless the system is not to be disturbed (`dnd`).
    Bell { dnd: bool, epoch: u64 },
    /// Find out what the notification server is and can do.
    Probe,
    /// Close `key`'s bubble, `how` says when.
    Withdraw { key: Key, how: Withdrawal },
    /// The waiter of `key`'s bubble `generation` ended; `gone`: it was clicked or closed.
    Ended { key: Key, generation: u64, gone: bool },
    #[cfg(test)]
    Settled(tokio::sync::oneshot::Sender<()>),
    /// Whether `key` has a bubble on screen.
    #[cfg(test)]
    Live(Key, tokio::sync::oneshot::Sender<bool>),
}

/// When a bubble is withdrawn.
enum Withdrawal {
    Always,
    /// When this read covers the message it shows, or (a conversation read) it shows a huddle's start.
    Read(Place),
    /// When this read covers the message it shows.
    Polled(Place),
}

impl Withdrawal {
    /// Whether it takes a bubble showing the message `at` (None: a huddle's start).
    fn takes(&self, at: Option<&Place>) -> bool {
        match (self, at) {
            (Withdrawal::Always, _) => true,
            (Withdrawal::Read(read) | Withdrawal::Polled(read), Some(at)) => at.read_by(read),
            (Withdrawal::Read(read), None) => read.thread.is_none(),
            (Withdrawal::Polled(_), None) => false,
        }
    }
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
        Notifier::with_waits(backend, DND_REUSE, DISCOVERY_RETRY)
    }

    fn with_waits(backend: Arc<dyn Backend>, reuse: std::time::Duration, retry: std::time::Duration) -> Notifier {
        Notifier { inner: Arc::new(Inner { backend, queue: OnceLock::new(), shared: Arc::new(Shared::new(reuse, retry)), focus: AtomicU8::new(TermFocus::Unknown as u8) }) }
    }

    fn send(&self, cmd: Cmd) {
        let queue = self.inner.queue.get_or_init(|| {
            let (tx, rx) = mpsc::unbounded_channel();
            match tokio::runtime::Handle::try_current() {
                Ok(rt) => drop(rt.spawn(serve(self.inner.backend.clone(), self.inner.shared.clone(), rx, tx.downgrade()))),
                Err(_) => tracing::warn!("notifications: no runtime to show them"),
            }
            tx
        });
        let _ = queue.send(cmd);
    }

    /// Show `note` with the settings' `sound` (None: silent), replacing its conversation's bubble; the notification
    /// carries the sound where the server can play it, else ttyak plays it. `on_click` runs when it is clicked (where
    /// ttyak hears of it: see [`Notifier::route`]).
    #[cfg(test)]
    pub fn show(&self, note: Note, sound: Option<Sound>, on_click: OnClick) {
        self.show_guarded(note, sound, Guard::default(), on_click);
    }

    /// As [`Notifier::show`], with the worker asking `guard` first.
    pub fn show_guarded(&self, note: Note, sound: Option<Sound>, guard: Guard, on_click: OnClick) {
        let at = note.target.ts.clone().map(|ts| Place { thread: note.target.thread_ts.clone(), ts });
        let tag = Some(osc99::id(&note.key));
        let bubble = Bubble { title: note.title, body: note.body, sound: sound.as_ref().map(|s| s.name.clone()), by: SoundBy::None, file: None, replaces: None, tag, clickable: true };
        let guard = Guard { epoch: self.inner.shared.epoch.load(Ordering::Relaxed), ..guard };
        self.send(Cmd::Show { key: Some(note.key), at, bubble, own: sound, guard, on_click: Some(on_click) });
    }

    /// Close `key`'s bubble, if one is up, and end its waiter.
    #[allow(dead_code)] // the reads withdraw through `read`; this one for the routes after this
    pub fn withdraw(&self, key: &Key) {
        self.send(Cmd::Withdraw { key: key.clone(), how: Withdrawal::Always });
    }

    /// `key`'s conversation (`thread` None) or one of its threads was read up to message `up_to`, here or on another
    /// client: its bubble is withdrawn when it shows a message there no newer than that, or (the conversation read) a
    /// huddle's start in it.
    pub fn read(&self, key: &Key, thread: Option<&str>, up_to: &str) {
        let place = Place { thread: thread.map(String::from), ts: up_to.to_string() };
        self.send(Cmd::Withdraw { key: key.clone(), how: Withdrawal::Read(place) });
    }

    /// A poll found `key`'s conversation read up to message `up_to`: as [`Notifier::read`], but a huddle's start stays
    /// (nobody read anything just now).
    pub fn polled_read(&self, key: &Key, up_to: &str) {
        self.send(Cmd::Withdraw { key: key.clone(), how: Withdrawal::Polled(Place { thread: None, ts: up_to.to_string() }) });
    }

    /// Show a bubble without any sound, ttyak's or the desktop's, that nothing replaces (the update notice: quiet hours
    /// do not hold it back, so it must never sound); `on_click` as in [`Notifier::show`].
    pub fn show_silent(&self, title: String, body: String, on_click: Option<OnClick>) {
        let bubble = Bubble { title, body, sound: None, by: SoundBy::None, file: None, replaces: None, tag: None, clickable: on_click.is_some() };
        self.send(Cmd::Show { key: None, at: None, bubble, own: None, guard: Guard::default(), on_click });
    }

    /// How bubbles, their sound (the settings' `sound`) and clicks go on this machine. (The server is looked for the first
    /// time this is asked: until it is found, the sound's route reads "not known yet".)
    #[allow(dead_code)] // read by F6, wired in after this
    pub fn route(&self, sound: &str) -> RouteInfo {
        let click = "opens it in ttyak";
        let environment = self.inner.shared.environment.lock().unwrap_or_else(|e| e.into_inner()).clone();
        if environment.is_none() && !self.inner.shared.probed.swap(true, Ordering::Relaxed) {
            self.send(Cmd::Probe);
        }
        let terminal = self.inner.shared.terminal.get();
        let sound_by = match environment.map(|e| if terminal.is_some() { osc99::sound_by(&e.capabilities, sound) } else { system_dnd::sound_route(&e.capabilities, sound) }) {
            None => "not known yet",
            Some(SoundBy::Notification) => "the notification",
            Some(SoundBy::Ttyak) if sound == "system" => "nobody (the server has no sound of its own to play)",
            Some(SoundBy::Ttyak) => "ttyak",
            Some(SoundBy::None) => "nobody (the sound is off)",
        };
        if let Some(terminal) = terminal {
            let shown_by = terminal.name.clone();
            let click = format!("opens it in ttyak; {} brings its window and tab forward", terminal.name);
            return RouteInfo { shown_by, sound_by: sound_by.into(), click };
        }
        #[cfg(target_os = "macos")]
        {
            let _ = click;
            let (shown_by, click) = mac_route(mac::sender(), mac::listening());
            return RouteInfo { shown_by, sound_by: sound_by.into(), click };
        }
        #[cfg(windows)]
        {
            let _ = click;
            let (shown_by, click) = windows_route(toast::setting());
            return RouteInfo { shown_by, sound_by: sound_by.into(), click };
        }
        #[cfg(all(unix, not(target_os = "macos")))]
        RouteInfo { shown_by: "the system's notifications".into(), sound_by: sound_by.into(), click: click.into() }
    }

    /// Ask for the bell (the run loop writes it between draws).
    pub fn ring(&self) {
        self.inner.shared.bell.store(self.inner.shared.epoch.load(Ordering::Relaxed) + 1, Ordering::Relaxed);
    }

    /// Ask for the bell unless the system is not to be disturbed: the worker asks it first.
    pub fn ring_unless_quiet(&self) {
        self.send(Cmd::Bell { dnd: true, epoch: self.inner.shared.epoch.load(Ordering::Relaxed) });
    }

    /// Whether a bell is due, once, and decided in the focus period it still is.
    pub fn take_bell(&self) -> bool {
        self.inner.shared.bell.swap(0, Ordering::Relaxed) == self.inner.shared.epoch.load(Ordering::Relaxed) + 1
    }

    /// The terminal reported its focus.
    pub fn set_focus(&self, focus: TermFocus) {
        if self.inner.focus.swap(focus as u8, Ordering::Relaxed) != focus as u8 {
            self.inner.shared.epoch.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// The terminal's focus as last reported.
    pub fn focus(&self) -> TermFocus {
        match self.inner.focus.load(Ordering::Relaxed) {
            f if f == TermFocus::Focused as u8 => TermFocus::Focused,
            f if f == TermFocus::Unfocused as u8 => TermFocus::Unfocused,
            _ => TermFocus::Unknown,
        }
    }

    /// From now on bubbles go through the terminal, `name`, itself (it answered the OSC 99 probe at start).
    pub fn use_terminal(&self, name: String) {
        self.use_terminal_with(name, CLOSE_GUARD);
    }

    fn use_terminal_with(&self, name: String, guard: std::time::Duration) {
        tracing::info!(terminal = name, "notifications shown by the terminal");
        let _ = self.inner.shared.terminal.set(Arc::new(Terminal::new(name, guard)));
    }

    /// An OSC reply from the terminal: a click on one of its bubbles (the bubble's click runs, as for the system's) or
    /// its closing.
    pub fn report(&self, osc: &str) {
        if let (Some(terminal), Some(report)) = (self.inner.shared.terminal.get(), osc99::report(osc)) {
            terminal.report(report);
        }
    }

    /// The sequences the terminal's bubbles wait to have written (the run loop writes them between draws).
    pub fn take_output(&self) -> Vec<String> {
        self.inner.shared.terminal.get().map(|t| t.take()).unwrap_or_default()
    }

    /// Once a sequence waits to be written (never without the terminal's bubbles).
    pub async fn output_due(&self) {
        match self.inner.shared.terminal.get() {
            Some(terminal) => terminal.due.notified().await,
            None => std::future::pending().await,
        }
    }
}

/// Windows' route for F6, given Windows' notification setting for PowerShell's app id (`ToastNotifier.Setting`; None:
/// not known): who shows the toasts and what a click does.
#[cfg(any(windows, test))]
fn windows_route(setting: Option<i32>) -> (String, String) {
    let off = match setting {
        Some(1) => Some("Windows PowerShell's notifications are turned off (Settings → System → Notifications)"),
        Some(2) => Some("notifications are turned off for this user (Settings → System → Notifications)"),
        Some(3) => Some("notifications are turned off by group policy"),
        Some(4) => Some("Windows PowerShell's app manifest turns notifications off"),
        _ => None,
    };
    if let Some(why) = off {
        return (format!("nothing: {why}"), "nothing to click".into());
    }
    let shown_by = "Windows' notifications, as Windows PowerShell's (ttyak has no app id of its own)".into();
    let click = "opens it in ttyak and brings the console's window forward, from the toast on screen (from the notification centre: not tried; likely not)".into();
    (shown_by, click)
}

/// macOS's route for F6: who shows the bubbles, sent as app `sender`, and what a click does (`clicks`: ttyak hears
/// them). Tried on macOS 26: Terminal's are shown, kitty's and Finder's refused.
#[cfg(any(target_os = "macos", test))]
fn mac_route(sender: &str, clicks: bool) -> (String, String) {
    let name = match sender {
        "com.apple.Terminal" => "Terminal",
        "com.googlecode.iterm2" => "iTerm2",
        "com.github.wez.wezterm" => "WezTerm",
        "com.mitchellh.ghostty" => "Ghostty",
        "net.kovidgoyal.kitty" => "kitty",
        _ => "Finder",
    };
    let refused = match name {
        "Finder" => Some("the terminal's app isn't known, and macOS 26 refuses notifications sent as Finder (macOS's default)"),
        "kitty" => Some("macOS 26 refuses notifications sent as kitty (in kitty itself ttyak uses kitty's own)"),
        _ => None,
    };
    if let Some(why) = refused {
        return (format!("nothing: {why}"), "nothing to click".into());
    }
    let tried = if name == "Terminal" { "" } else { "; not tried: macOS 26 may refuse them" };
    let shown_by = format!("macOS's notifications, as {name}'s (only where {name}'s notifications are allowed in System Settings, and not while the screen is shared unless allowed there{tried})");
    // (Terminal's and iTerm2's tab by AppleScript; the others' app with its front window. Tried: Terminal.)
    let forward = if matches!(name, "Terminal" | "iTerm2") { format!("{name}'s tab") } else { name.to_string() };
    let click = if clicks { format!("opens it in ttyak and brings {forward} forward (up to 32 waiting at once)") } else { format!("brings {name} forward (ttyak isn't told)") };
    (shown_by, click)
}

/// The worker: shows, replaces and closes bubbles in the order asked, on `backend`.
async fn serve(backend: Arc<dyn Backend>, shared: Arc<Shared>, mut queue: mpsc::UnboundedReceiver<Cmd>, back: mpsc::WeakUnboundedSender<Cmd>) {
    let mut live: HashMap<Key, Live> = HashMap::new();
    let mut generation = 0;
    while let Some(cmd) = queue.recv().await {
        match cmd {
            Cmd::Show { key, at, mut bubble, own, guard, on_click } => {
                // The system's do-not-disturb holds back all of it; else the bell, and the sound where the server will
                // not play it.
                let environment = if bubble.sound.is_some() || guard.dnd { Some(environment(&backend, &shared).await) } else { None };
                if guard.dnd && held_back(&backend, &shared, environment.as_ref()).await {
                    continue;
                }
                if guard.bell {
                    ring_in(&shared, guard.epoch);
                }
                let terminal = shared.terminal.get().cloned();
                if let (Some(sound), Some(environment), Some(_)) = (&bubble.sound, &environment, &terminal) {
                    bubble.by = osc99::sound_by(&environment.capabilities, sound);
                } else if let (Some(sound), Some(environment)) = (&bubble.sound, &environment) {
                    bubble.by = system_dnd::sound_route(&environment.capabilities, sound);
                    if bubble.by == SoundBy::Notification && sound != "system" {
                        bubble.file = backend.sound_file(sound);
                        // (A sound that cannot be written for the server is played by ttyak rather than lost.)
                        if bubble.file.is_none() {
                            bubble.by = SoundBy::Ttyak;
                        }
                    }
                }
                if let (SoundBy::Ttyak, Some(own)) = (bubble.by, own) {
                    backend.play(own);
                }
                bubble.replaces = key.as_ref().and_then(|k| live.get(k)).map(|o| o.shown.id());
                // A server that can't replace in place (macOS): the older bubble goes first, so the newer one is the
                // only one left (they may show the same).
                if let Some(old) = key.as_ref().and_then(|k| live.get(k)).filter(|o| o.shown.closed_to_replace() && terminal.is_none()) {
                    if tokio::time::timeout(SERVER_WAIT, old.shown.close()).await.is_err() {
                        tracing::warn!("replacing a notification: the notification server did not answer");
                    }
                }
                let shown = match &terminal {
                    Some(t) => Ok(Ok(t.show(key.as_ref().map(osc99::id).unwrap_or_else(|| format!("ttyak-{}", generation + 1)), &bubble))),
                    None => tokio::time::timeout(SERVER_WAIT, backend.show(bubble)).await,
                };
                let shown = match shown {
                    Ok(Ok(shown)) => shown,
                    // The bubble it would replace stays as it was (still withdrawn and clicked as before).
                    Ok(Err(e)) => {
                        tracing::warn!("notification failed: {e}");
                        *shared.environment.lock().unwrap_or_else(|e| e.into_inner()) = None;
                        continue;
                    }
                    Err(_) => {
                        tracing::warn!("notification failed: the notification server did not answer");
                        *shared.environment.lock().unwrap_or_else(|e| e.into_inner()) = None;
                        continue;
                    }
                };
                // The replaced bubble's waiter ends: the server keeps its id for the new one. (A click on the old one
                // that came in just now is lost with it.)
                if let Some(w) = key.as_ref().and_then(|k| live.remove(k)).and_then(|o| o.waiter) {
                    w.abort();
                }
                generation += 1;
                let ended = key.clone().map(|k| (back.clone(), k, generation));
                let on = if terminal.is_some() { ClickedOn::Terminal } else { ClickedOn::System };
                let waiter = on_click.map(|f| tokio::spawn(wait(shown.clone(), on, f, ended)).abort_handle());
                if let Some(key) = key {
                    live.insert(key, Live { shown, at, generation, waiter });
                }
            }
            Cmd::Bell { dnd, epoch } => {
                let environment = if dnd { Some(environment(&backend, &shared).await) } else { None };
                if !(dnd && held_back(&backend, &shared, environment.as_ref()).await) {
                    ring_in(&shared, epoch);
                }
            }
            Cmd::Probe => drop(environment(&backend, &shared).await),
            Cmd::Withdraw { key, how } => {
                let covered = live.get(&key).is_some_and(|l| how.takes(l.at.as_ref()));
                if let Some(l) = covered.then(|| live.remove(&key)).flatten() {
                    if let Some(w) = l.waiter {
                        w.abort();
                    }
                    if tokio::time::timeout(SERVER_WAIT, l.shown.close()).await.is_err() {
                        tracing::warn!("withdrawing a notification: the notification server did not answer");
                    }
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
            #[cfg(test)]
            Cmd::Live(key, answer) => drop(answer.send(live.contains_key(&key))),
        }
    }
}

/// The bell is due, if the focus period `epoch` it was decided in is still the current one.
fn ring_in(shared: &Shared, epoch: u64) {
    if shared.epoch.load(Ordering::Relaxed) == epoch {
        shared.bell.store(epoch + 1, Ordering::Relaxed);
    }
}

/// What the notification server is and can do: as found before, else asked (for at most [`SERVER_WAIT`]; a server that
/// does not answer is taken for one that can do nothing, and not asked again for [`DISCOVERY_RETRY`]).
async fn environment(backend: &Arc<dyn Backend>, shared: &Shared) -> Environment {
    let nothing = || Environment { server: Server::Other(String::new()), capabilities: Vec::new() };
    if let Some(known) = shared.environment.lock().unwrap_or_else(|e| e.into_inner()).clone() {
        return known;
    }
    let failed = *shared.discovery_failed.lock().unwrap_or_else(|e| e.into_inner());
    if failed.is_some_and(|at| at.elapsed() < shared.retry) {
        return nothing();
    }
    let found = match tokio::time::timeout(SERVER_WAIT, backend.environment()).await {
        Ok(Ok(found)) => Some(found),
        Ok(Err(e)) => {
            tracing::debug!("notification server: {e}");
            None
        }
        Err(_) => {
            tracing::debug!("notification server: no answer");
            None
        }
    };
    *shared.discovery_failed.lock().unwrap_or_else(|e| e.into_inner()) = found.is_none().then(std::time::Instant::now);
    if let Some(found) = &found {
        tracing::debug!(server = ?found.server, capabilities = ?found.capabilities, "notification server");
    }
    *shared.environment.lock().unwrap_or_else(|e| e.into_inner()) = found.clone();
    found.unwrap_or_else(nothing)
}

/// Whether the system says notifications are suppressed (not being able to tell counts as not). An answer younger than
/// [`DND_REUSE`] stands for the next event.
async fn held_back(backend: &Arc<dyn Backend>, shared: &Shared, environment: Option<&Environment>) -> bool {
    let Some(environment) = environment else { return false };
    let recent = *shared.dnd.lock().unwrap_or_else(|e| e.into_inner());
    let answer = match recent {
        Some((at, answer)) if at.elapsed() < shared.reuse => answer,
        _ => {
            let answer = backend.suppressed(environment.server.clone()).await;
            *shared.dnd.lock().unwrap_or_else(|e| e.into_inner()) = Some((std::time::Instant::now(), answer));
            answer
        }
    };
    if answer == Some(true) {
        tracing::debug!(server = ?environment.server, "the system's do-not-disturb is on: nothing fired");
    }
    answer == Some(true)
}

/// Wait for a click on `shown` (at most [`CLICK_WAIT`]) and run `on_click` for it, told it is `on`'s bubble; then tell
/// the worker (`ended`).
async fn wait(shown: Arc<dyn Shown>, on: ClickedOn, on_click: OnClick, ended: Option<(mpsc::WeakUnboundedSender<Cmd>, Key, u64)>) {
    let gone = match tokio::time::timeout(CLICK_WAIT, shown.clicked()).await {
        Ok(true) => {
            on_click(on);
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

/// Fire what `out` lets through for `note`: the bubble (replacing its conversation's) with its sound, and the bell.
/// With `follow_system_dnd` the worker asks the system first and fires nothing while it is not to be disturbed.
pub fn interrupt(notifier: &Notifier, note: Note, out: &Outputs, settings: &super::settings::Settings, on_click: OnClick) {
    let dnd = settings.notifications.follow_system_dnd;
    if out.bubble {
        notifier.show_guarded(note, out.sound.then(|| Sound::of(settings)), Guard { dnd, bell: out.bell, ..Guard::default() }, on_click);
    } else if out.bell {
        if dnd {
            notifier.ring_unless_quiet();
        } else {
            notifier.ring();
        }
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
        Show { replaces: Option<u32>, title: String, body: String, sound: Option<String>, by: SoundBy, file: Option<std::path::PathBuf>, clickable: bool },
        Close(u32),
        /// ttyak's own player was asked for this sound.
        Play(String),
    }

    /// Records what it is asked; clicks on demand; can be down.
    pub struct Server {
        calls: Mutex<Vec<Call>>,
        last_id: AtomicU32,
        down: AtomicBool,
        hung: AtomicBool,
        environment: Mutex<Environment>,
        suppressed: Mutex<Option<bool>>,
        /// How often the system was asked about do-not-disturb, and the notification server about itself.
        asked: AtomicU32,
        looked: AtomicU32,
        /// The server cannot be found / the sound cannot be written as a file.
        no_server: AtomicBool,
        no_files: AtomicBool,
        /// Bubbles can't be replaced in place (macOS): each gets an id of its own and is closed to be replaced.
        closes_to_replace: AtomicBool,
        clicks: Mutex<HashMap<u32, Arc<tokio::sync::Notify>>>,
    }

    impl Default for Server {
        /// A server that cannot play sounds and does not suppress anything.
        fn default() -> Server {
            Server {
                calls: Mutex::default(),
                last_id: AtomicU32::default(),
                down: AtomicBool::default(),
                hung: AtomicBool::default(),
                environment: Mutex::new(Environment { server: system_dnd::Server::Other("fake".into()), capabilities: Vec::new() }),
                suppressed: Mutex::new(Some(false)),
                asked: AtomicU32::default(),
                looked: AtomicU32::default(),
                no_server: AtomicBool::default(),
                no_files: AtomicBool::default(),
                closes_to_replace: AtomicBool::default(),
                clicks: Mutex::default(),
            }
        }
    }

    impl Server {
        /// What the server offers from now on.
        pub fn set_capabilities(&self, capabilities: &[&str]) {
            self.environment.lock().unwrap().capabilities = capabilities.iter().map(|c| c.to_string()).collect();
        }

        /// How often the system was asked about do-not-disturb.
        pub fn asked(&self) -> u32 {
            self.asked.load(Ordering::Relaxed)
        }

        /// How often the server was looked for.
        pub fn looked(&self) -> u32 {
            self.looked.load(Ordering::Relaxed)
        }

        /// From now on the server cannot be found (`no_server`), or can.
        pub fn set_no_server(&self, no_server: bool) {
            self.no_server.store(no_server, Ordering::Relaxed);
        }

        /// From now on bubbles can't be replaced in place (`closes`), or can.
        pub fn set_closes_to_replace(&self, closes: bool) {
            self.closes_to_replace.store(closes, Ordering::Relaxed);
        }

        /// From now on the built-in sounds cannot be written as files, or can.
        pub fn set_no_files(&self, no_files: bool) {
            self.no_files.store(no_files, Ordering::Relaxed);
        }

        /// What the system says about do-not-disturb from now on (None: cannot be told).
        pub fn set_suppressed(&self, suppressed: Option<bool>) {
            *self.suppressed.lock().unwrap() = suppressed;
        }

        pub fn calls(&self) -> Vec<Call> {
            self.calls.lock().unwrap().clone()
        }

        /// From now on every bubble fails (`down`), or works again.
        pub fn set_down(&self, down: bool) {
            self.down.store(down, Ordering::Relaxed);
        }

        /// From now on the server never answers a bubble (`hung`), or answers again.
        pub fn set_hung(&self, hung: bool) {
            self.hung.store(hung, Ordering::Relaxed);
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
                server.calls.lock().unwrap().push(Call::Show { replaces: b.replaces, title: b.title, body: b.body, sound: b.sound, by: b.by, file: b.file, clickable: b.clickable });
                if server.down.load(Ordering::Relaxed) {
                    anyhow::bail!("no notification server");
                }
                if server.hung.load(Ordering::Relaxed) {
                    std::future::pending::<()>().await;
                }
                let in_place = b.replaces.filter(|_| !server.closes_to_replace.load(Ordering::Relaxed));
                let id = in_place.unwrap_or_else(|| server.last_id.fetch_add(1, Ordering::Relaxed) + 1);
                Ok(Arc::new(OnScreen { id, server }) as Arc<dyn Shown>)
            })
        }

        fn environment(&self) -> Fut<'static, anyhow::Result<Environment>> {
            self.0.looked.fetch_add(1, Ordering::Relaxed);
            let found = self.0.environment.lock().unwrap().clone();
            let missing = self.0.no_server.load(Ordering::Relaxed);
            Box::pin(async move { if missing { anyhow::bail!("no bus") } else { Ok(found) } })
        }

        fn suppressed(&self, _: system_dnd::Server) -> Fut<'static, Option<bool>> {
            self.0.asked.fetch_add(1, Ordering::Relaxed);
            let answer = *self.0.suppressed.lock().unwrap();
            Box::pin(async move { answer })
        }

        fn play(&self, sound: Sound) {
            self.0.calls.lock().unwrap().push(Call::Play(sound.name));
        }

        fn sound_file(&self, name: &str) -> Option<std::path::PathBuf> {
            (!self.0.no_files.load(Ordering::Relaxed)).then(|| std::path::PathBuf::from(format!("/fake/{name}.wav")))
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

        fn closed_to_replace(&self) -> bool {
            self.server.closes_to_replace.load(Ordering::Relaxed)
        }
    }

    impl Notifier {
        /// A notifier on a server in memory.
        pub fn fake() -> (Notifier, Arc<Server>) {
            let server = Arc::new(Server::default());
            (Notifier::with_waits(Arc::new(Memory(server.clone())), std::time::Duration::ZERO, std::time::Duration::ZERO), server)
        }

        /// As [`Notifier::fake`], with the real waits for reusing an answer and retrying a lost server.
        pub fn fake_remembering() -> (Notifier, Arc<Server>) {
            let server = Arc::new(Server::default());
            (Notifier::with(Arc::new(Memory(server.clone()))), server)
        }

        /// Once everything asked so far is done.
        pub async fn settled(&self) {
            let (done, settled) = tokio::sync::oneshot::channel();
            self.send(Cmd::Settled(done));
            let _ = tokio::time::timeout(std::time::Duration::from_secs(5), settled).await;
        }

        /// Whether `key`'s bubble is still known (shown, replaced and withdrawn as one), once everything asked so far is
        /// done.
        pub async fn live(&self, key: &Key) -> bool {
            let (answer, live) = tokio::sync::oneshot::channel();
            self.send(Cmd::Live(key.clone(), answer));
            tokio::time::timeout(std::time::Duration::from_secs(5), live).await.ok().and_then(|r| r.ok()).unwrap_or(false)
        }

        /// As [`Notifier::use_terminal`], a close reported within `guard` of a newer bubble with its id taken for the older's.
        pub fn use_terminal_guarding(&self, name: String, guard: std::time::Duration) {
            self.use_terminal_with(name, guard);
        }

        /// How many click waiters the terminal's bubbles keep.
        pub fn waiting(&self) -> usize {
            self.inner.shared.terminal.get().map_or(0, |t| t.waiting.lock().unwrap().len())
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
        let quiet = notice("t", "b", None, SoundBy::None, None);
        let loud = notice("t", "b", Some("system"), SoundBy::Notification, None);
        assert_eq!((quiet.summary.as_str(), quiet.body.as_str(), quiet.appname.as_str()), ("t", "b", "ttyak"));
        #[cfg(all(unix, not(target_os = "macos")))]
        {
            use notify_rust::Hint;
            assert!(quiet.hints.contains(&Hint::SuppressSound(true)) && !quiet.hints.iter().any(|h| matches!(h, Hint::SoundName(_))), "{:?}", quiet.hints);
            assert!(!loud.hints.contains(&Hint::SuppressSound(true)) && loud.hints.iter().any(|h| matches!(h, Hint::SoundName(_))), "{:?}", loud.hints);
        }
        let _ = loud;
    }

    /// A panic in ttyak ends the main thread's run loop and goes on from there (it once spun forever).
    #[cfg(target_os = "macos")]
    #[test]
    fn a_panic_ends_the_run_loop() {
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let panicked = std::panic::catch_unwind(|| mac::with_run_loop(|| -> u8 { panic!("in ttyak") })).is_err();
            let _ = tx.send((panicked, mac::with_run_loop(|| 7).ok()));
        });
        assert_eq!(rx.recv_timeout(std::time::Duration::from_secs(10)), Ok((true, Some(7))), "returned, and the panic went on");
    }

    #[test]
    fn windows_toasts() {
        // One tag per conversation (a newer toast with it replaces the older; withdrawing takes it by it), at most 64 long.
        assert_eq!(toast::tag(&osc99::id(&Key { workspace: 0, channel: "C1".into() })), "w0-C1");
        assert_eq!(toast::tag(&"x".repeat(80)), "x".repeat(64), "Windows' longest tag");
        let toast = |title: &str, body: &str, audio: &str| format!(r#"<toast><visual><binding template="ToastGeneric"><text>{title}</text><text>{body}</text></binding></visual>{audio}</toast>"#);
        assert_eq!(toast::xml("Bob", "hi", false), toast("Bob", "hi", r#"<audio silent="true"/>"#));
        assert_eq!(toast::xml("Bob", "hi", true), toast("Bob", "hi", ""), "the desktop's own sound");
        // Each profile's toasts in a group of their own: two ttyaks don't replace or withdraw each other's.
        assert_eq!((toast::group("default"), toast::group("b")), ("ttyak-default".to_string(), "ttyak-b".to_string()));
        assert_eq!(toast::group("a b/ü_-1"), "ttyak-ab_-1", "only what a tag may hold");
        assert_eq!(toast::group(""), "ttyak");
        assert_eq!(toast::group(&"p".repeat(100)).len(), 64, "Windows' longest group");
        assert_eq!(toast::xml("<A&B>", "\"x\" 'y'\u{1}\tz\n2", false), toast("&lt;A&amp;B&gt;", "&quot;x&quot; &apos;y&apos;\tz\n2", r#"<audio silent="true"/>"#), "escaped; control characters XML forbids left out");
    }

    /// The toast carries its tag and group, and its XML loads; withdrawing one that isn't there is no error.
    #[cfg(windows)]
    #[test]
    fn a_toast_is_named_by_its_tag() {
        let t = toast::build("Bob", "a <b> & c", false, Some("w0-C1"), &toast::group("b")).unwrap();
        assert_eq!((t.Tag().unwrap().to_string(), t.Group().unwrap().to_string()), ("w0-C1".to_string(), "ttyak-b".to_string()));
        assert_eq!(t.Content().unwrap().GetXml().unwrap().to_string(), toast::xml("Bob", "a <b> & c", false));
        let untagged = toast::build("t", "b", true, None, "ttyak-b").unwrap();
        assert_eq!(untagged.Tag().unwrap().to_string(), "");
        toast::withdraw(None, Some("w0-ttyak-test-none"), "ttyak-b").unwrap();
        println!("Windows' notification setting for PowerShell's app id: {:?}", toast::setting());
        assert!(toast::setting().is_some());
    }

    /// A toast reaches the notification centre under its tag, and withdrawing takes it out by it. Shown without its popup,
    /// so nothing comes on screen: `ttyak.exe live_toast_history --ignored --nocapture` (Windows).
    #[cfg(windows)]
    #[test]
    #[ignore = "puts a toast (without its popup) into the notification centre, and takes it out"]
    fn live_toast_history() {
        use windows::UI::Notifications::{ToastNotification, ToastNotificationManager};
        use windows::core::HSTRING;
        const TAG: &str = "w9-TTYAKTEST";
        let (ours, theirs) = (toast::group("ttyaktest"), toast::group("ttyaktest2"));
        let in_centre = |group: &str| -> usize {
            let history = ToastNotificationManager::History().unwrap().GetHistoryWithId(&HSTRING::from(toast::APP_ID)).unwrap();
            history.into_iter().filter(|t| t.Tag().is_ok_and(|t| t == TAG) && t.Group().is_ok_and(|g| g == group)).count()
        };
        let show = |body: &str, group: &str| -> ToastNotification {
            let t = toast::build("ttyak test", body, false, Some(TAG), group).unwrap();
            t.SetSuppressPopup(true).unwrap();
            ToastNotificationManager::CreateToastNotifierWithId(&HSTRING::from(toast::APP_ID)).unwrap().Show(&t).unwrap();
            t
        };
        show("another profile's, the same tag", &theirs);
        show("taken out at once", &ours);
        let shown = in_centre(&ours);
        let again = show("the same tag", &ours);
        let replaced = in_centre(&ours);
        toast::withdraw(Some(&again), Some(TAG), &ours).unwrap();
        let (left, other) = (in_centre(&ours), in_centre(&theirs));
        toast::withdraw(None, Some(TAG), &theirs).unwrap();
        let other_left = in_centre(&theirs);
        println!("in the notification centre: shown {shown}, after a second with its tag {replaced}, withdrawn {left}; the other profile's {other}, then {other_left}");
        assert_eq!((shown, replaced, left, other, other_left), (1, 1, 0, 1, 0));
    }

    #[test]
    fn what_windows_shows() {
        // Enabled, or the setting can't be read: shown as Windows PowerShell's.
        for setting in [Some(0), None] {
            let (shown_by, click) = windows_route(setting);
            assert!(shown_by.contains("as Windows PowerShell's") && click.starts_with("opens it in ttyak"), "{setting:?}: {shown_by} / {click}");
        }
        // Turned off in Windows: toasts would vanish without a trace, so F6 says why.
        for (setting, why) in [(1, "Windows PowerShell"), (2, "for this user"), (3, "group policy"), (4, "manifest")] {
            let (shown_by, click) = windows_route(Some(setting));
            assert!(shown_by.starts_with("nothing: ") && shown_by.contains(why) && click == "nothing to click", "{setting}: {shown_by} / {click}");
        }
    }

    #[test]
    fn macos_bubbles() {
        // Only "system" is the desktop's own sound; ttyak plays any other itself.
        assert_eq!(mac_sound(Some("system"), SoundBy::Notification), Some(super::super::sound::system_sound_name()));
        assert_eq!(mac_sound(Some("system"), SoundBy::None), None);
        assert_eq!(mac_sound(Some("chime"), SoundBy::Ttyak), None);
        assert_eq!(mac_sound(None, SoundBy::Notification), None);
        // So many clicks are waited for at once, then none until one is done.
        let count = std::sync::atomic::AtomicUsize::new(0);
        assert!(take_place(&count, 2) && take_place(&count, 2) && !take_place(&count, 2));
        count.fetch_sub(1, Ordering::Relaxed);
        assert!(take_place(&count, 2));
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    #[test]
    fn off_mutes_the_server() {
        use notify_rust::Hint;
        let file = std::path::Path::new("/fake/chime.wav");
        for by in [SoundBy::None, SoundBy::Ttyak, SoundBy::Notification] {
            let off = notice("t", "b", None, by, None);
            assert!(off.hints.contains(&Hint::SuppressSound(true)), "{by:?}: {:?}", off.hints);
        }
        // ttyak plays it itself: the server stays quiet.
        let own = notice("t", "b", Some("chime"), SoundBy::Ttyak, None);
        assert!(own.hints.contains(&Hint::SuppressSound(true)) && !own.hints.iter().any(|h| matches!(h, Hint::SoundFile(_) | Hint::SoundName(_))), "{:?}", own.hints);
        // The server plays it: the file of the built-in sound, or the desktop's own name.
        let loud = notice("t", "b", Some("chime"), SoundBy::Notification, Some(file));
        assert!(!loud.hints.contains(&Hint::SuppressSound(true)) && loud.hints.contains(&Hint::SoundFile("/fake/chime.wav".into())), "{:?}", loud.hints);
        let named = notice("t", "b", Some("system"), SoundBy::Notification, None);
        assert!(named.hints.iter().any(|h| matches!(h, Hint::SoundName(_))) && !named.hints.iter().any(|h| matches!(h, Hint::SoundFile(_))), "{:?}", named.hints);
    }

    fn sound(name: &str) -> Option<Sound> {
        Some(Sound { name: name.into(), device: None, volume: 100 })
    }

    #[tokio::test]
    async fn the_sound_goes_with_the_notification() {
        let (n, server) = Notifier::fake();
        // A server that plays sounds: the notification carries it, ttyak plays nothing.
        server.set_capabilities(&["body", "sound"]);
        n.show(note("D1", "1.0", None, "one"), sound("chime"), Box::new(|_| {}));
        n.settled().await;
        let calls = server.calls();
        // (macOS's and Windows' bubbles carry only the desktop's own sound: ttyak plays the chime there.)
        let carried = cfg!(all(unix, not(target_os = "macos")));
        let first = if carried {
            matches!(calls.as_slice(), [fake::Call::Show { sound: Some(s), by: SoundBy::Notification, .. }] if s == "chime")
        } else {
            matches!(calls.as_slice(), [fake::Call::Play(p), fake::Call::Show { by: SoundBy::Ttyak, .. }] if p == "chime")
        };
        assert!(first, "{calls:?}");
        assert_eq!(n.route("chime").sound_by, if carried { "the notification" } else { "ttyak" });
        // Off: nobody, whatever the server can do.
        n.show(note("D3", "3.0", None, "three"), None, Box::new(|_| {}));
        n.settled().await;
        let calls = server.calls();
        assert!(matches!(&calls[if carried { 1 } else { 2 }..], [fake::Call::Show { sound: None, by: SoundBy::None, .. }]), "{calls:?}");
        // One that cannot play: ttyak's own player, before the bubble.
        let (n, server) = Notifier::fake();
        n.show(note("D2", "2.0", None, "two"), sound("chime"), Box::new(|_| {}));
        n.settled().await;
        let calls = server.calls();
        assert!(matches!(calls.as_slice(), [fake::Call::Play(p), fake::Call::Show { by: SoundBy::Ttyak, .. }] if p == "chime"), "{calls:?}");
    }

    #[tokio::test]
    async fn a_bell_decided_in_an_earlier_focus_period_is_dropped() {
        let (n, _server) = Notifier::fake();
        n.set_focus(TermFocus::Unfocused);
        n.show_guarded(note("D1", "1.0", None, "one"), None, Guard { dnd: true, bell: true, ..Guard::default() }, Box::new(|_| {}));
        n.ring_unless_quiet();
        // Focus came back and went again before the worker had asked the system.
        n.set_focus(TermFocus::Focused);
        n.set_focus(TermFocus::Unfocused);
        n.settled().await;
        assert!(!n.take_bell(), "that bell was for the period before");
        n.show_guarded(note("D1", "2.0", None, "two"), None, Guard { dnd: true, bell: true, ..Guard::default() }, Box::new(|_| {}));
        n.settled().await;
        assert!(n.take_bell(), "one decided in this period rings");
    }

    #[tokio::test]
    async fn route_tells_who_plays_the_sound() {
        let (n, server) = Notifier::fake();
        server.set_capabilities(&["sound"]);
        assert_eq!(n.route("chime").sound_by, "not known yet", "the server is looked for now");
        n.settled().await;
        assert_eq!(n.route("system").sound_by, "the notification");
        assert_eq!(n.route("off").sound_by, "nobody (the sound is off)");
        let (n, _) = Notifier::fake();
        n.route("chime");
        n.settled().await;
        assert_eq!(n.route("chime").sound_by, "ttyak");
        assert_eq!(n.route("system").sound_by, "nobody (the server has no sound of its own to play)", "ttyak has no system sound");
    }

    #[tokio::test]
    async fn a_burst_asks_the_system_once() {
        let (n, server) = Notifier::fake_remembering();
        let guard = Guard { dnd: true, bell: false, ..Guard::default() };
        for i in 0..5 {
            n.show_guarded(note("D1", &format!("{i}.0"), None, "x"), None, guard, Box::new(|_| {}));
        }
        n.settled().await;
        assert_eq!((server.asked(), server.looked()), (1, 1), "one question and one look for five events");
        let (n, server) = Notifier::fake();
        for i in 0..3 {
            n.show_guarded(note("D1", &format!("{i}.0"), None, "x"), None, guard, Box::new(|_| {}));
        }
        n.settled().await;
        assert_eq!(server.asked(), 3, "no reuse without the wait");
    }

    #[tokio::test]
    async fn a_lost_server_is_not_asked_for_again_at_once() {
        let (n, server) = Notifier::fake_remembering();
        server.set_no_server(true);
        for i in 0..3 {
            n.show_guarded(note("D1", &format!("{i}.0"), None, "x"), sound("chime"), Guard::default(), Box::new(|_| {}));
        }
        n.settled().await;
        assert_eq!(server.looked(), 1, "looked for once, remembered");
        let plays = server.calls().iter().filter(|c| matches!(c, fake::Call::Play(_))).count();
        assert_eq!(plays, 3, "meanwhile ttyak plays the sounds itself");
    }

    #[tokio::test]
    async fn a_sound_that_cannot_be_a_file_is_played_by_ttyak() {
        let (n, server) = Notifier::fake();
        server.set_capabilities(&["sound"]);
        server.set_no_files(true);
        n.show(note("D1", "1.0", None, "one"), sound("chime"), Box::new(|_| {}));
        n.settled().await;
        let calls = server.calls();
        assert!(matches!(calls.as_slice(), [fake::Call::Play(_), fake::Call::Show { by: SoundBy::Ttyak, file: None, .. }]) || !cfg!(all(unix, not(target_os = "macos"))), "{calls:?}");
    }

    #[tokio::test]
    async fn the_systems_do_not_disturb_holds_back_all_of_it() {
        let (n, server) = Notifier::fake();
        let guard = Guard { dnd: true, bell: true, ..Guard::default() };
        server.set_suppressed(Some(true));
        n.show_guarded(note("D1", "1.0", None, "one"), sound("chime"), guard, Box::new(|_| {}));
        n.ring_unless_quiet();
        n.settled().await;
        assert!(server.calls().is_empty() && !n.take_bell(), "no bubble, no sound, no bell: {:?}", server.calls());
        // Not asked when the setting says not to follow it.
        n.show_guarded(note("D1", "2.0", None, "two"), sound("chime"), Guard { dnd: false, bell: true, ..Guard::default() }, Box::new(|_| {}));
        n.settled().await;
        assert_eq!(server.calls().len(), 2, "the sound and the bubble");
        assert!(n.take_bell());
        // Can't tell, or not suppressed: it goes through.
        for answer in [None, Some(false)] {
            server.set_suppressed(answer);
            let before = server.calls().len();
            n.show_guarded(note("D2", "3.0", None, "three"), None, guard, Box::new(|_| {}));
            n.ring_unless_quiet();
            n.settled().await;
            assert_eq!(server.calls().len(), before + 1, "{answer:?}");
            assert!(n.take_bell(), "{answer:?}");
        }
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
        fake::Call::Show { replaces, title: "Bob".into(), body: body.into(), sound: None, by: SoundBy::None, file: None, clickable: true }
    }

    /// What each click ran, by the body of the note it was for.
    fn recorder() -> (Arc<Mutex<Vec<&'static str>>>, impl Fn(&'static str) -> OnClick) {
        let ran = Arc::new(Mutex::new(Vec::new()));
        let r = ran.clone();
        (ran, move |what| {
            let r = r.clone();
            Box::new(move |_| r.lock().unwrap().push(what)) as OnClick
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
        n.show(note("D1", "4.0", None, "three"), Some(Sound { name: "system".into(), device: None, volume: 100 }), on_click("three"));
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

    #[test]
    fn what_macos_shows() {
        let (shown, click) = mac_route("com.apple.Terminal", true);
        assert!(shown.starts_with("macOS's notifications, as Terminal's (only where Terminal's notifications are allowed") && !shown.contains("not tried"), "{shown}");
        assert_eq!(click, "opens it in ttyak and brings Terminal's tab forward (up to 32 waiting at once)");
        assert_eq!(mac_route("com.apple.Terminal", false).1, "brings Terminal forward (ttyak isn't told)");
        assert!(mac_route("com.googlecode.iterm2", true).0.contains("not tried"));
        assert_eq!(mac_route("com.mitchellh.ghostty", true).1, "opens it in ttyak and brings Ghostty forward (up to 32 waiting at once)", "the app, not a tab");
        let (shown, click) = mac_route("com.apple.Finder", true);
        assert!(shown.starts_with("nothing: the terminal's app isn't known") && click == "nothing to click", "{shown}");
        assert!(mac_route("net.kovidgoyal.kitty", true).0.starts_with("nothing: macOS 26 refuses notifications sent as kitty"));
    }

    #[tokio::test]
    async fn a_server_that_cannot_replace_closes_the_older_bubble_first() {
        // macOS's older API: the older bubble is taken out of the notification centre, then the newer one shown.
        let (n, server) = Notifier::fake();
        server.set_closes_to_replace(true);
        let (clicked, on_click) = recorder();
        n.show(note("D1", "1.0", None, "one"), None, on_click("one"));
        n.show(note("D1", "2.0", None, "two"), None, on_click("two"));
        n.settled().await;
        assert_eq!(server.calls(), [shown(None, "one"), fake::Call::Close(1), shown(Some(1), "two")]);
        server.click(2);
        n.settled_after(|| !clicked.lock().unwrap().is_empty()).await;
        assert_eq!(*clicked.lock().unwrap(), ["two"]);
        // Read: withdrawn the same way.
        n.show(note("D1", "3.0", None, "three"), None, Box::new(|_| {}));
        n.read(&Key { workspace: 0, channel: "D1".into() }, None, "3.0");
        n.settled().await;
        assert_eq!(server.calls()[3..], [shown(None, "three"), fake::Call::Close(3)]);
    }

    #[tokio::test]
    async fn reading_withdraws_it() {
        let (n, server) = Notifier::fake();
        let key = Key { workspace: 0, channel: "D1".into() };
        n.show(note("D1", "1.0", None, "one"), None, Box::new(|_| {}));
        n.withdraw(&key);
        n.withdraw(&key);
        n.settled().await;
        assert_eq!(server.calls(), [shown(None, "one"), fake::Call::Close(1)], "closed once");
        // Only a read that covers what it shows withdraws it.
        n.show(note("D1", "5.0", None, "five"), None, Box::new(|_| {}));
        n.read(&key, None, "4.0");
        n.read(&key, Some("5.0"), "9.0");
        n.read(&Key { workspace: 1, channel: "D1".into() }, None, "9.0");
        n.settled().await;
        assert_eq!(server.calls().len(), 3, "an older mark, a thread, another workspace: {:?}", server.calls());
        n.read(&key, None, "5.0");
        n.settled().await;
        assert_eq!(server.calls().pop(), Some(fake::Call::Close(2)));
        // A reply's bubble goes with its thread, not with the conversation.
        n.show(note("D1", "6.0", Some("3.0"), "a reply"), None, Box::new(|_| {}));
        n.read(&key, None, "9.0");
        n.settled().await;
        assert_eq!(server.calls().len(), 5);
        n.read(&key, Some("3.0"), "6.0");
        n.settled().await;
        assert_eq!(server.calls().pop(), Some(fake::Call::Close(3)));
    }

    #[tokio::test]
    async fn a_click_that_ended_before_a_newer_bubble_leaves_it() {
        let (n, server) = Notifier::fake();
        let (clicked, on_click) = recorder();
        let key = Key { workspace: 0, channel: "D1".into() };
        n.show(note("D1", "1.0", None, "one"), None, on_click("one"));
        n.settled().await;
        // Clicked as a newer message comes: the old waiter's end reaches the worker after the newer bubble.
        server.click(1);
        n.show(note("D1", "2.0", None, "two"), None, on_click("two"));
        n.settled_after(|| !clicked.lock().unwrap().is_empty()).await;
        assert_eq!(*clicked.lock().unwrap(), ["one"]);
        n.withdraw(&key);
        n.settled().await;
        assert_eq!(server.calls(), [shown(None, "one"), shown(Some(1), "two"), fake::Call::Close(1)], "the newer bubble is still known");
    }

    #[tokio::test]
    async fn a_failed_replace_keeps_the_bubble() {
        let (n, server) = Notifier::fake();
        let (clicked, on_click) = recorder();
        n.show(note("D1", "1.0", None, "one"), None, on_click("one"));
        n.settled().await;
        server.set_down(true);
        n.show(note("D1", "2.0", None, "two"), None, on_click("two"));
        n.settled().await;
        server.set_down(false);
        server.click(1);
        n.settled_after(|| !clicked.lock().unwrap().is_empty()).await;
        assert_eq!(*clicked.lock().unwrap(), ["one"], "the old bubble still answers its click");
        n.show(note("D1", "3.0", None, "three"), None, on_click("three"));
        n.show(note("D1", "4.0", None, "four"), None, on_click("four"));
        n.settled().await;
        server.set_down(true);
        n.show(note("D1", "5.0", None, "five"), None, on_click("five"));
        n.read(&Key { workspace: 0, channel: "D1".into() }, None, "4.0");
        n.settled().await;
        assert_eq!(server.calls().pop(), Some(fake::Call::Close(2)), "and is still withdrawn: {:?}", server.calls());
    }

    #[tokio::test]
    async fn reading_the_conversation_takes_a_huddle_start() {
        let (n, server) = Notifier::fake();
        let key = Key { workspace: 0, channel: "D1".into() };
        let huddle = Note { key: key.clone(), title: "Bob started a huddle".into(), body: "b".into(), target: Target { channel: "D1".into(), ts: None, thread_ts: None } };
        n.show(huddle, None, Box::new(|_| {}));
        n.read(&key, Some("1.0"), "9.0");
        n.polled_read(&key, "9.0");
        n.settled().await;
        assert_eq!(server.calls().len(), 1, "a thread read or a poll leaves it: {:?}", server.calls());
        n.read(&key, None, "1.0");
        n.settled().await;
        assert_eq!(server.calls().pop(), Some(fake::Call::Close(1)));
    }

    #[tokio::test]
    async fn a_hung_server_holds_back_nothing() {
        let (n, server) = Notifier::fake();
        server.set_hung(true);
        n.show(note("D1", "1.0", None, "stuck"), None, Box::new(|_| {}));
        n.settled().await;
        server.set_hung(false);
        n.show(note("D2", "2.0", None, "next"), None, Box::new(|_| {}));
        tokio::time::timeout(std::time::Duration::from_secs(3), n.settled()).await.unwrap();
        assert_eq!(server.calls(), [shown(None, "stuck"), shown(None, "next")]);
    }

    #[tokio::test]
    async fn no_server_no_panic() {
        let (n, server) = Notifier::fake();
        server.set_down(true);
        n.show(note("D1", "1.0", None, "one"), None, Box::new(|_| {}));
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

    /// A notifier whose bubbles kitty shows, on a server in memory (for the system's do-not-disturb and sound).
    fn on_terminal() -> (Notifier, Arc<fake::Server>) {
        let (n, server) = Notifier::fake();
        n.use_terminal("kitty".into());
        (n, server)
    }

    #[tokio::test]
    async fn the_terminal_replaces_and_closes_by_the_conversations_id() {
        let (n, server) = on_terminal();
        let key = Key { workspace: 0, channel: "D1".into() };
        n.show(note("D1", "1.0", None, "one"), None, Box::new(|_| {}));
        n.show(note("D1", "2.0", None, "two"), None, Box::new(|_| {}));
        n.settled().await;
        let out = n.take_output();
        assert_eq!(out.len(), 2, "{out:?}");
        assert!(out.iter().all(|s| s.contains("i=w0-D1:") && s.contains("a=focus,report")), "{out:?}");
        assert!(n.take_output().is_empty(), "written once");
        assert!(server.calls().is_empty(), "not through the system's notifier: {:?}", server.calls());
        n.read(&key, None, "1.0");
        n.settled().await;
        assert!(n.take_output().is_empty(), "the read does not cover what it shows");
        n.read(&key, None, "2.0");
        n.read(&key, None, "2.0");
        n.settled().await;
        assert_eq!(n.take_output(), [osc99::close("w0-D1")], "closed once");
        // The update notice: an id of its own each time.
        n.show_silent("ttyak 9.9.9 is out".into(), "b".into(), None);
        n.show_silent("ttyak 9.9.9 is out".into(), "b".into(), None);
        n.settled().await;
        let out = n.take_output();
        assert!(out[0].contains("i=ttyak-") && out[1].contains("i=ttyak-") && out[0][..30] != out[1][..30], "{out:?}");
    }

    #[tokio::test]
    async fn the_terminals_report_runs_the_click() {
        let (n, _server) = on_terminal();
        let (clicked, on_click) = recorder();
        n.show(note("D1", "1.0", None, "one"), None, on_click("one"));
        n.show(note("D2", "2.0", None, "two"), None, on_click("two"));
        n.settled().await;
        n.report("99;i=w0-D9;");
        n.report("99;i=ttyak-probe:p=?;a=focus,report:p=title,body");
        n.report("99;i=w0-D2;");
        n.settled_after(|| !clicked.lock().unwrap().is_empty()).await;
        assert_eq!(*clicked.lock().unwrap(), ["two"]);
        // kitty says the clicked bubble closed too: nothing more.
        n.report("99;i=w0-D2:p=close;");
        // Closed by the user: gone, so a read has nothing to take away, and a click on it later does nothing.
        let _ = n.take_output();
        n.report("99;i=w0-D1:p=close;");
        gone(&n, &Key { workspace: 0, channel: "D1".into() }).await;
        n.report("99;i=w0-D1;");
        n.read(&Key { workspace: 0, channel: "D1".into() }, None, "9.0");
        n.read(&Key { workspace: 0, channel: "D2".into() }, None, "9.0");
        n.settled().await;
        assert_eq!(*clicked.lock().unwrap(), ["two"]);
        assert!(n.take_output().is_empty(), "nothing left to close: {:?}", n.take_output());
        // A replaced bubble's waiter has ended: the click goes to the newer one.
        n.show(note("D1", "3.0", None, "three"), None, on_click("three"));
        n.show(note("D1", "4.0", None, "four"), None, on_click("four"));
        n.settled().await;
        n.report("99;i=w0-D1;1");
        n.settled_after(|| clicked.lock().unwrap().len() == 2).await;
        assert_eq!(*clicked.lock().unwrap(), ["two", "four"]);
    }

    #[tokio::test]
    async fn the_terminal_carries_only_the_desktops_sound() {
        let (n, server) = on_terminal();
        server.set_capabilities(&["sound"]);
        n.show(note("D1", "1.0", None, "one"), sound("chime"), Box::new(|_| {}));
        n.settled().await;
        assert_eq!(server.calls(), [fake::Call::Play("chime".into())], "kitty cannot play ttyak's sounds: ttyak does");
        assert!(n.take_output()[0].contains(&format!("s={}", super::super::clipboard::base64(b"silent"))));
        n.show(note("D2", "2.0", None, "two"), sound("system"), Box::new(|_| {}));
        n.show(note("D3", "3.0", None, "three"), None, Box::new(|_| {}));
        n.settled().await;
        assert_eq!(server.calls().len(), 1, "the desktop's sound goes with the bubble: {:?}", server.calls());
        let out = n.take_output();
        assert!(out[0].contains(&format!("s={}", super::super::clipboard::base64(b"system"))) && out[1].contains(&format!("s={}", super::super::clipboard::base64(b"silent"))), "{out:?}");
        assert_eq!(n.route("chime"), RouteInfo { shown_by: "kitty".into(), sound_by: "ttyak".into(), click: "opens it in ttyak; kitty brings its window and tab forward".into() });
        assert_eq!(n.route("system").sound_by, "the notification");
        // The system's do-not-disturb holds it back as well.
        server.set_suppressed(Some(true));
        n.show_guarded(note("D4", "4.0", None, "four"), sound("chime"), Guard { dnd: true, bell: true, ..Guard::default() }, Box::new(|_| {}));
        n.settled().await;
        assert!(n.take_output().is_empty() && server.calls().len() == 1 && !n.take_bell());
    }

    /// Once `key`'s bubble is forgotten (its waiter's end reached the worker), at most 5 s.
    async fn gone(n: &Notifier, key: &Key) {
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(5);
        while n.live(key).await && tokio::time::Instant::now() < deadline {
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
    }

    #[tokio::test]
    async fn an_untracked_close_is_no_close() {
        let (n, _server) = on_terminal();
        let (clicked, on_click) = recorder();
        let key = Key { workspace: 0, channel: "D1".into() };
        // kitty on macOS says, right after showing it, that it cannot tell when it closes.
        n.show(note("D1", "1.0", None, "one"), None, on_click("one"));
        n.settled().await;
        n.report("99;i=w0-D1:p=close;untracked");
        n.settled().await;
        assert!(n.live(&key).await, "still on screen");
        n.report("99;i=w0-D1;");
        n.settled_after(|| !clicked.lock().unwrap().is_empty()).await;
        assert_eq!(*clicked.lock().unwrap(), ["one"], "still clickable");
        n.show(note("D1", "2.0", None, "two"), None, on_click("two"));
        n.settled().await;
        n.report("99;i=w0-D1:p=close;untracked");
        n.settled().await;
        let _ = n.take_output();
        n.read(&key, None, "2.0");
        n.settled().await;
        assert_eq!(n.take_output(), [osc99::close("w0-D1")], "still withdrawn by a read");
    }

    #[tokio::test]
    async fn a_close_crossing_a_replace_leaves_the_newer_bubble() {
        let (n, _server) = Notifier::fake();
        n.use_terminal_guarding("kitty".into(), std::time::Duration::from_secs(10));
        let (clicked, on_click) = recorder();
        let key = Key { workspace: 0, channel: "D1".into() };
        n.show(note("D1", "1.0", None, "one"), None, on_click("one"));
        n.settled().await;
        n.show(note("D1", "2.0", None, "two"), None, on_click("two"));
        n.settled().await;
        let _ = n.take_output();
        // The first was dismissed just as the second went out: kitty's close is about the first.
        n.report("99;i=w0-D1:p=close;");
        n.settled().await;
        assert!(n.live(&key).await, "the newer one is still known");
        n.report("99;i=w0-D1;");
        n.settled_after(|| !clicked.lock().unwrap().is_empty()).await;
        assert_eq!(*clicked.lock().unwrap(), ["two"]);
    }

    #[tokio::test]
    async fn only_bubbles_that_wait_for_a_click_keep_a_waiter() {
        let (n, _server) = on_terminal();
        let key = Key { workspace: 0, channel: "D1".into() };
        n.show_silent("ttyak 9.9.9 is out".into(), "b".into(), None);
        n.show_silent("ttyak 9.9.9 is out".into(), "b".into(), None);
        n.settled().await;
        assert_eq!(n.waiting(), 0, "the update notice without a click");
        n.show(note("D1", "1.0", None, "one"), None, Box::new(|_| {}));
        n.show(note("D1", "2.0", None, "two"), None, Box::new(|_| {}));
        n.settled().await;
        assert_eq!(n.waiting(), 1, "one per bubble on screen");
        n.withdraw(&key);
        n.settled().await;
        assert_eq!(n.waiting(), 0);
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
