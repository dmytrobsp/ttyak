//! The notification server: what it can do (play the sound itself) and whether it is holding notifications back (the
//! system's do-not-disturb), so ttyak neither chimes beside a silenced popup nor interrupts a quiet desktop.

use std::time::Duration;

/// Who plays a notification's sound.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SoundBy {
    /// The notification carries it: the system's do-not-disturb silences both.
    Notification,
    /// ttyak's own player, beside the notification.
    Ttyak,
    /// Nobody: the sound is off.
    None,
}

/// Who really plays `sound` on this machine: [`sound_by`], except where the route can carry only the desktop's own sound
/// (macOS and Windows toasts, until their routes play files). One decision for the worker and for F6's route.
pub fn sound_route(capabilities: &[String], sound: &str) -> SoundBy {
    let by = sound_by(capabilities, sound);
    if cfg!(not(all(unix, not(target_os = "macos")))) && by == SoundBy::Notification && sound != "system" {
        return SoundBy::Ttyak;
    }
    by
}

/// Who plays `sound` (the settings' value) given the server's `capabilities`.
pub fn sound_by(capabilities: &[String], sound: &str) -> SoundBy {
    if sound == "off" {
        SoundBy::None
    } else if capabilities.iter().any(|c| c == "sound") {
        SoundBy::Notification
    } else {
        SoundBy::Ttyak
    }
}

/// The notification server, by what ttyak can ask of it.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))] // only Linux asks
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Server {
    Swaync,
    Mako,
    Dunst,
    Gnome,
    Kde,
    /// Any other, by the name it gave.
    Other(String),
}

/// The server called `name` in its `GetServerInformation`.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))] // only Linux asks
pub fn server(name: &str) -> Server {
    let n = name.to_lowercase();
    if n.contains("swaync") || n.contains("swaynotificationcenter") {
        Server::Swaync
    } else if n == "mako" {
        Server::Mako
    } else if n.contains("dunst") {
        Server::Dunst
    } else if n.contains("gnome") {
        Server::Gnome
    } else if n.contains("plasma") || n.contains("kde") {
        Server::Kde
    } else {
        Server::Other(name.to_string())
    }
}

/// What the notification server is and can do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Environment {
    pub server: Server,
    pub capabilities: Vec<String>,
}

/// How long one question to the system takes at most (an internal wait: a hung desktop must not hold back the
/// notification it is asked about).
pub const ASK_WAIT: Duration = Duration::from_millis(if cfg!(test) { 300 } else { 1000 });

/// The session bus, connected once and kept (every question would otherwise connect anew); forgotten when a call on it
/// fails, so the next question connects again.
#[cfg(target_os = "linux")]
static BUS: std::sync::Mutex<Option<zbus::Connection>> = std::sync::Mutex::new(None);

#[cfg(target_os = "linux")]
async fn bus() -> zbus::Result<zbus::Connection> {
    if let Some(conn) = BUS.lock().unwrap_or_else(|e| e.into_inner()).clone() {
        return Ok(conn);
    }
    let conn = zbus::Connection::session().await?;
    *BUS.lock().unwrap_or_else(|e| e.into_inner()) = Some(conn.clone());
    Ok(conn)
}

#[cfg(target_os = "linux")]
fn forget_bus() {
    *BUS.lock().unwrap_or_else(|e| e.into_inner()) = None;
}

/// The server answering on the session bus. (Windows and macOS: their toasts carry the sound, which [`sound_route`]
/// narrows to the desktop's own; nothing to ask.)
pub async fn discover() -> anyhow::Result<Environment> {
    #[cfg(target_os = "linux")]
    {
        let found = discover_on_bus().await;
        if found.is_err() {
            forget_bus();
        }
        found
    }
    #[cfg(not(target_os = "linux"))]
    {
        Ok(Environment { server: Server::Other(String::new()), capabilities: vec!["sound".into()] })
    }
}

#[cfg(target_os = "linux")]
async fn discover_on_bus() -> anyhow::Result<Environment> {
    let conn = bus().await?;
    let notifications = zbus::Proxy::new(&conn, "org.freedesktop.Notifications", "/org/freedesktop/Notifications", "org.freedesktop.Notifications").await?;
    let capabilities: Vec<String> = notifications.call("GetCapabilities", &()).await?;
    let (name, ..): (String, String, String, String) = notifications.call("GetServerInformation", &()).await?;
    Ok(Environment { server: server(&name), capabilities })
}

/// Whether `server` holds notifications back right now; None: it cannot be told (unknown server, not answering in
/// [`ASK_WAIT`], an answer that makes no sense; macOS's Focus, which has no unprivileged way to be read, and
/// Windows'). Asked per for-you event.
pub async fn suppressed(server: &Server) -> Option<bool> {
    within(ASK_WAIT, ask(server)).await
}

/// `ask`'s answer, None if it takes longer than `wait`.
async fn within(wait: Duration, ask: impl std::future::Future<Output = Option<bool>>) -> Option<bool> {
    tokio::time::timeout(wait, ask).await.ok().flatten()
}

async fn ask(server: &Server) -> Option<bool> {
    match server {
        Server::Swaync => swaync().await,
        Server::Mako => run("makoctl", &["mode"]).await.map(|o| mako_dnd(&o)),
        Server::Dunst => run("dunstctl", &["is-paused"]).await.and_then(|o| dunst_paused(&o)),
        Server::Gnome => run("gsettings", &["get", "org.gnome.desktop.notifications", "show-banners"]).await.and_then(|o| gnome_banners_off(&o)),
        Server::Kde => kde().await,
        Server::Other(_) => None,
    }
}

/// `makoctl mode` lists the active modes, one per line; the modes named `do-not-disturb` and `dnd` count.
fn mako_dnd(out: &str) -> bool {
    out.lines().any(|l| matches!(l.trim(), "do-not-disturb" | "dnd"))
}

/// `dunstctl is-paused` prints `true` or `false`.
fn dunst_paused(out: &str) -> Option<bool> {
    match out.trim() {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

/// `gsettings get … show-banners` prints `true` or `false`; banners off is do-not-disturb.
fn gnome_banners_off(out: &str) -> Option<bool> {
    dunst_paused(out).map(|banners| !banners)
}

/// The command's stdout when it ran and succeeded (killed when the question times out).
async fn run(program: &str, args: &[&str]) -> Option<String> {
    let out = tokio::process::Command::new(program).args(args).stdin(std::process::Stdio::null()).stderr(std::process::Stdio::null()).kill_on_drop(true).output().await.ok()?;
    out.status.success().then(|| String::from_utf8_lossy(&out.stdout).into_owned())
}

#[cfg(target_os = "linux")]
async fn swaync() -> Option<bool> {
    let conn = bus().await.ok()?;
    let cc = zbus::Proxy::new(&conn, "org.erikreider.swaync.cc", "/org/erikreider/swaync/cc", "org.erikreider.swaync.cc").await.ok()?;
    let answer = cc.call("GetDnd", &()).await;
    if matches!(&answer, Err(zbus::Error::InputOutput(_))) {
        forget_bus();
    }
    answer.ok()
}

#[cfg(target_os = "linux")]
async fn kde() -> Option<bool> {
    let conn = bus().await.ok()?;
    let n = zbus::Proxy::new(&conn, "org.freedesktop.Notifications", "/org/freedesktop/Notifications", "org.freedesktop.Notifications").await.ok()?;
    let answer = n.get_property("Inhibited").await;
    if matches!(&answer, Err(zbus::Error::InputOutput(_))) {
        forget_bus();
    }
    answer.ok()
}

#[cfg(not(target_os = "linux"))]
async fn swaync() -> Option<bool> {
    None
}

#[cfg(not(target_os = "linux"))]
async fn kde() -> Option<bool> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn caps(c: &[&str]) -> Vec<String> {
        c.iter().map(|c| c.to_string()).collect()
    }

    #[test]
    fn who_plays_the_sound() {
        assert_eq!(sound_by(&caps(&["body", "sound"]), "chime"), SoundBy::Notification);
        assert_eq!(sound_by(&caps(&["sound"]), "system"), SoundBy::Notification);
        assert_eq!(sound_by(&caps(&["body", "actions"]), "chime"), SoundBy::Ttyak);
        assert_eq!(sound_by(&[], "chime"), SoundBy::Ttyak);
        assert_eq!(sound_by(&caps(&["sound"]), "off"), SoundBy::None);
        assert_eq!(sound_by(&[], "off"), SoundBy::None);
    }

    #[test]
    fn servers_by_name() {
        assert_eq!(server("SwayNotificationCenter"), Server::Swaync);
        assert_eq!(server("mako"), Server::Mako);
        assert_eq!(server("dunst"), Server::Dunst);
        assert_eq!(server("gnome-shell"), Server::Gnome);
        assert_eq!(server("GNOME Shell"), Server::Gnome);
        assert_eq!(server("Plasma"), Server::Kde);
        assert_eq!(server("KDE"), Server::Kde);
        assert_eq!(server("Xfce Notify Daemon"), Server::Other("Xfce Notify Daemon".into()));
    }

    #[test]
    fn what_the_tools_say() {
        assert!(mako_dnd("default\ndo-not-disturb\n") && !mako_dnd("default\n") && !mako_dnd(""));
        assert!(mako_dnd("default\ndnd\n") && !mako_dnd("dnd-ish\nwork\n"), "a mode named dnd counts too");
        assert_eq!((dunst_paused("true\n"), dunst_paused("false\n"), dunst_paused("")), (Some(true), Some(false), None));
        assert_eq!((gnome_banners_off("false\n"), gnome_banners_off("true\n"), gnome_banners_off("uint32 3")), (Some(true), Some(false), None));
    }

    #[tokio::test]
    async fn a_question_that_takes_too_long_is_no_answer() {
        let started = std::time::Instant::now();
        assert_eq!(within(ASK_WAIT, std::future::pending()).await, None);
        assert!(started.elapsed() >= ASK_WAIT && started.elapsed() < ASK_WAIT * 5, "{:?}", started.elapsed());
        assert_eq!(within(ASK_WAIT, async { Some(true) }).await, Some(true));
    }

    #[test]
    fn the_route_is_what_the_worker_does() {
        let sound = caps(&["sound"]);
        assert_eq!(sound_route(&sound, "system"), SoundBy::Notification);
        assert_eq!(sound_route(&sound, "off"), SoundBy::None);
        assert_eq!(sound_route(&[], "chime"), SoundBy::Ttyak);
        // Only freedesktop servers play a built-in sound as a file; macOS and Windows play ttyak's.
        let chime = if cfg!(all(unix, not(target_os = "macos"))) { SoundBy::Notification } else { SoundBy::Ttyak };
        assert_eq!(sound_route(&sound, "chime"), chime);
    }

    #[tokio::test]
    async fn an_unknown_server_cannot_be_told() {
        assert_eq!(suppressed(&Server::Other("x".into())).await, None);
    }

    /// Reads this desktop's server and its do-not-disturb (read-only): `cargo test live_server -- --ignored --nocapture`.
    #[tokio::test]
    #[ignore = "asks the real notification server"]
    async fn live_server() {
        let found = discover().await.unwrap();
        println!("{found:?} -> suppressed {:?}", suppressed(&found.server).await);
    }
}
