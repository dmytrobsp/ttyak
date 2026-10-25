//! Bringing the terminal's window forward after a click on the system notifier's bubble (the terminal's own bubbles,
//! OSC 99, bring it forward themselves): tmux's window, pane and client, the terminal's remote control (kitty, WezTerm),
//! the window manager (Hyprland, sway, niri, KDE's KWin, X11). Where nothing can (GNOME on Wayland), nothing happens.

use std::collections::HashMap;
use std::io::Read;
use std::time::{Duration, Instant};

/// How long one command (or the X server) may take before it is given up (internal).
const COMMAND_WAIT: Duration = Duration::from_secs(2);

/// The name the KWin script is loaded under.
const KWIN_SCRIPT: &str = "ttyak-raise";

/// [`describe`]'s words (through `tr`).
pub const VIA: &str = "via {routes}";
pub const AND: &str = "{first} and {last}";
pub const CANNOT: &str = "the window can't be raised on this desktop";
pub const WORDS: &[&str] = &[VIA, AND, CANNOT];

/// A window the window manager lists, and the process it belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Window {
    pub id: u64,
    pub pid: u32,
}

/// What the raise is picked from: ttyak's environment variables, its parent processes (nearest first), the OS, and
/// what the tools the variables point at answered ([`env`] asks them).
#[derive(Clone, Debug, Default)]
pub struct Env {
    pub vars: HashMap<String, String>,
    pub ancestors: Vec<u32>,
    pub os: &'static str,
    /// kitty answered `kitty @ ls` at `KITTY_LISTEN_ON` (remote control is on).
    pub kitty_answers: bool,
    /// The windows of the window manager the variables point at (Hyprland's clients, sway's tree, niri's windows, the X
    /// server's `_NET_CLIENT_LIST`).
    pub windows: Vec<Window>,
    /// Inside tmux: the client last active on ttyak's session — its tty, and the outer terminal as that client sees it
    /// (its variables and parents).
    pub tmux_client: Option<(String, Box<Env>)>,
}

/// How the window is brought forward.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Raise {
    /// tmux: ttyak's window and pane selected, the client (its tty) switched to them; then the outer terminal.
    Tmux { pane: String, client: String, then: Box<Raise> },
    /// kitty's remote control at `to`: ttyak's window and tab focused; then the window manager.
    Kitty { window: String, to: String, then: Box<Raise> },
    /// WezTerm's: ttyak's pane activated (through `socket`, when known); then the window manager.
    Wezterm { pane: String, socket: Option<String>, then: Box<Raise> },
    Hyprland { pid: u32 },
    Sway { pid: u32 },
    Niri { window: u64 },
    /// KDE: a KWin script activates the window of the first of `pids` that has one (Plasma 6, else 5).
    Kwin { pids: Vec<u32>, plasma6: bool },
    /// EWMH `_NET_ACTIVE_WINDOW`.
    X11 { window: u64 },
    None,
}

/// The desktop the variables point at, first found in this order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Desktop {
    Hyprland,
    Sway,
    Niri,
    Kde { plasma6: bool },
    X11,
    Other,
}

/// One step of a raise.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Step {
    /// A command, run without a shell; `var`: a variable set for it.
    Run(Vec<String>, Option<(&'static str, String)>),
    /// The KWin script for these processes (Plasma 6, else 5).
    Kwin(Vec<u32>, bool),
    /// `_NET_ACTIVE_WINDOW` for this window.
    X11(u64),
}

/// This process's environment, parents and OS, with the answers of the tools they point at (each asked for at most
/// [`COMMAND_WAIT`]).
pub fn env() -> Env {
    let (vars, ancestors): (HashMap<String, String>, _) = (std::env::vars().collect(), parents(std::process::id()));
    if let (Some(_), Some(pane)) = (var(&vars, "TMUX"), var(&vars, "TMUX_PANE")) {
        // Inside tmux only the outer terminal is asked (the pane's variables are the tmux server's, maybe stale).
        let tmux_client = tmux_client(pane).map(|(pid, tty)| (tty, Box::new(probed(environ(pid), parents(pid)))));
        return Env { vars, ancestors, os: std::env::consts::OS, tmux_client, ..Env::default() };
    }
    probed(vars, ancestors)
}

/// The best raise for `env`, in the order: tmux, kitty, WezTerm, then the window manager.
pub fn detect(env: &Env) -> Raise {
    let var = |name: &str| var(&env.vars, name).map(String::from);
    if let (Some(_), Some(pane)) = (var("TMUX"), var("TMUX_PANE")) {
        // Without a client the session is on no screen.
        return match &env.tmux_client {
            Some((client, outer)) => Raise::Tmux { pane, client: client.clone(), then: Box::new(detect(outer)) },
            None => Raise::None,
        };
    }
    // (Never through the terminal's own tty: ttyak reads it.)
    if let (Some(window), Some(to), true) = (var("KITTY_WINDOW_ID"), var("KITTY_LISTEN_ON"), env.kitty_answers) {
        return Raise::Kitty { window, to, then: Box::new(window_manager(env)) };
    }
    if let Some(pane) = var("WEZTERM_PANE") {
        return Raise::Wezterm { pane, socket: var("WEZTERM_UNIX_SOCKET"), then: Box::new(window_manager(env)) };
    }
    window_manager(env)
}

/// The window manager's raise, of the nearest parent of ttyak that has a window.
fn window_manager(env: &Env) -> Raise {
    let window = || env.ancestors.iter().find_map(|pid| env.windows.iter().find(|w| w.pid == *pid));
    match desktop(&env.vars, env.os) {
        Desktop::Hyprland => window().map_or(Raise::None, |w| Raise::Hyprland { pid: w.pid }),
        Desktop::Sway => window().map_or(Raise::None, |w| Raise::Sway { pid: w.pid }),
        Desktop::Niri => window().map_or(Raise::None, |w| Raise::Niri { window: w.id }),
        Desktop::Kde { plasma6 } => Raise::Kwin { pids: env.ancestors.clone(), plasma6 },
        Desktop::X11 => var(&env.vars, "WINDOWID").and_then(|w| w.parse().ok()).or_else(|| window().map(|w| w.id)).map_or(Raise::None, |window| Raise::X11 { window }),
        Desktop::Other => Raise::None,
    }
}

/// Which desktop `vars` point at (macOS and Windows: none of these).
fn desktop(vars: &HashMap<String, String>, os: &str) -> Desktop {
    let set = |name: &str| var(vars, name).is_some();
    if matches!(os, "macos" | "windows") {
        Desktop::Other
    } else if set("HYPRLAND_INSTANCE_SIGNATURE") {
        Desktop::Hyprland
    } else if set("SWAYSOCK") {
        Desktop::Sway
    } else if set("NIRI_SOCKET") {
        Desktop::Niri
    } else if let Some(version) = var(vars, "KDE_SESSION_VERSION") {
        Desktop::Kde { plasma6: version != "5" }
    } else if set("DISPLAY") && !set("WAYLAND_DISPLAY") {
        Desktop::X11
    } else {
        Desktop::Other
    }
}

/// Variable `name` of `vars`, when not empty.
fn var<'a>(vars: &'a HashMap<String, String>, name: &str) -> Option<&'a str> {
    vars.get(name).map(String::as_str).filter(|v| !v.is_empty())
}

/// An environment of `vars` and `ancestors`, with what the tools they point at answer.
fn probed(vars: HashMap<String, String>, ancestors: Vec<u32>) -> Env {
    let os = std::env::consts::OS;
    let kitty_answers = match (var(&vars, "KITTY_WINDOW_ID"), var(&vars, "KITTY_LISTEN_ON")) {
        (Some(window), Some(to)) => output(&argv(&["kitty", "@", "--to", to, "ls", "--match", &format!("id:{window}")]), None).is_some(),
        _ => false,
    };
    let listed = |command: &[&str], parse: fn(&str) -> Vec<Window>| output(&argv(command), None).map(|out| parse(&out)).unwrap_or_default();
    let windows = match desktop(&vars, os) {
        Desktop::Hyprland => listed(&["hyprctl", "-j", "clients"], hyprland_windows),
        Desktop::Sway => listed(&["swaymsg", "-r", "-t", "get_tree"], sway_windows),
        Desktop::Niri => listed(&["niri", "msg", "--json", "windows"], niri_windows),
        Desktop::X11 if var(&vars, "WINDOWID").is_none() => x11_windows(),
        _ => Vec::new(),
    };
    Env { vars, ancestors, os, kitty_answers, windows, tmux_client: None }
}

/// The tmux client last active on the session of `pane`: its pid and tty.
fn tmux_client(pane: &str) -> Option<(u32, String)> {
    let session = output(&argv(&["tmux", "display-message", "-p", "-t", pane, "#{session_id}"]), None)?;
    let list = output(&argv(&["tmux", "list-clients", "-t", session.trim(), "-F", "#{client_activity} #{client_pid} #{client_tty}"]), None)?;
    latest_client(&list)
}

/// Bring the window forward by `r`; each command at most [`COMMAND_WAIT`], failures logged.
pub fn raise(r: &Raise) {
    for step in steps(r) {
        match step {
            Step::Run(argv, var) => {
                if let Some(out) = output(&argv, var.as_ref()).filter(|o| !o.trim().is_empty()) {
                    tracing::debug!(command = argv[0], answer = out.trim(), "raise");
                }
            }
            Step::Kwin(pids, plasma6) => kwin(&pids, plasma6),
            Step::X11(window) => x11_activate(window),
        }
    }
}

/// Bring the window forward in the background (the tools are asked anew each time: windows and tmux clients come and
/// go), never holding up the caller.
pub fn bring_forward() {
    std::thread::spawn(|| {
        let r = detect(&env());
        tracing::debug!(?r, route = describe(&r), "bringing the window forward");
        raise(&r);
    });
}

/// How the window is brought forward, for F6: "via kitty and Hyprland", or that it can't be.
pub fn describe(r: &Raise) -> String {
    use super::i18n::tr;
    let mut names = Vec::new();
    let mut at = r;
    loop {
        let (name, then) = match at {
            Raise::Tmux { then, .. } => ("tmux", Some(then)),
            Raise::Kitty { then, .. } => ("kitty", Some(then)),
            Raise::Wezterm { then, .. } => ("WezTerm", Some(then)),
            Raise::Hyprland { .. } => ("Hyprland", None),
            Raise::Sway { .. } => ("sway", None),
            Raise::Niri { .. } => ("niri", None),
            Raise::Kwin { .. } => ("KWin", None),
            Raise::X11 { .. } => ("X11", None),
            Raise::None => break,
        };
        names.push(name);
        match then {
            Some(then) => at = then,
            None => break,
        }
    }
    let routes = match names.split_last() {
        None => return tr(CANNOT).into_owned(),
        Some((last, [])) => last.to_string(),
        Some((last, first)) => tr(AND).replace("{first}", &first.join(", ")).replace("{last}", last),
    };
    tr(VIA).replace("{routes}", &routes)
}

/// What runs for `r`, in order.
fn steps(r: &Raise) -> Vec<Step> {
    let run = |args: &[&str]| Step::Run(argv(args), None);
    match r {
        Raise::Tmux { pane, client, then } => {
            let mut steps = vec![run(&["tmux", "select-window", "-t", pane]), run(&["tmux", "select-pane", "-t", pane]), run(&["tmux", "switch-client", "-c", client, "-t", pane])];
            steps.extend(self::steps(then));
            steps
        }
        Raise::Kitty { window, to, then } => [vec![run(&["kitty", "@", "--to", to, "focus-window", "--match", &format!("id:{window}")])], self::steps(then)].concat(),
        Raise::Wezterm { pane, socket, then } => {
            let activate = Step::Run(argv(&["wezterm", "cli", "activate-pane", "--pane-id", pane]), socket.clone().map(|s| ("WEZTERM_UNIX_SOCKET", s)));
            [vec![activate], self::steps(then)].concat()
        }
        Raise::Hyprland { pid } => vec![run(&["hyprctl", "dispatch", "focuswindow", &format!("pid:{pid}")])],
        Raise::Sway { pid } => vec![run(&["swaymsg", &format!("[pid={pid}] focus")])],
        Raise::Niri { window } => vec![run(&["niri", "msg", "action", "focus-window", "--id", &window.to_string()])],
        Raise::Kwin { pids, plasma6 } => vec![Step::Kwin(pids.clone(), *plasma6)],
        Raise::X11 { window } => vec![Step::X11(*window)],
        Raise::None => Vec::new(),
    }
}

fn argv(args: &[&str]) -> Vec<String> {
    args.iter().map(|a| a.to_string()).collect()
}

/// Run `argv` without a shell (`var` set for it) for at most [`COMMAND_WAIT`]: what it printed, when it succeeded.
fn output(argv: &[String], var: Option<&(&'static str, String)>) -> Option<String> {
    use std::process::{Command, Stdio};
    let (program, args) = argv.split_first()?;
    let mut command = Command::new(program);
    command.args(args).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::null());
    if let Some((name, value)) = var {
        command.env(name, value);
    }
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(e) => {
            tracing::debug!("raise: {program}: {e}");
            return None;
        }
    };
    let mut stdout = child.stdout.take()?;
    let (tx, rx) = std::sync::mpsc::channel();
    // Read on the side: a long answer (sway's tree) would otherwise fill the pipe and never end.
    std::thread::spawn(move || {
        let mut text = String::new();
        let _ = stdout.read_to_string(&mut text);
        let _ = tx.send(text);
    });
    let deadline = Instant::now() + COMMAND_WAIT;
    let text = rx.recv_timeout(COMMAND_WAIT).ok();
    loop {
        match child.try_wait() {
            Ok(Some(status)) if status.success() => return text,
            Ok(Some(status)) => {
                tracing::debug!("raise: {program} failed ({status})");
                return None;
            }
            Ok(None) if text.is_some() && Instant::now() < deadline => std::thread::sleep(Duration::from_millis(10)),
            _ => {
                tracing::debug!("raise: {program} did not answer within {} s", COMMAND_WAIT.as_secs());
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
        }
    }
}

/// Run `job` for at most [`COMMAND_WAIT`] (it is left behind if it hangs).
#[cfg(target_os = "linux")]
fn bounded<T: Send + 'static>(job: impl FnOnce() -> Option<T> + Send + 'static) -> Option<T> {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || drop(tx.send(job())));
    rx.recv_timeout(COMMAND_WAIT).ok().flatten()
}

/// Hyprland's `hyprctl -j clients`.
fn hyprland_windows(json: &str) -> Vec<Window> {
    let clients: Vec<serde_json::Value> = serde_json::from_str(json).unwrap_or_default();
    clients
        .iter()
        .filter_map(|c| {
            let id = u64::from_str_radix(c["address"].as_str()?.trim_start_matches("0x"), 16).ok()?;
            Some(Window { id, pid: u32::try_from(c["pid"].as_u64()?).ok()? })
        })
        .collect()
}

/// sway's `swaymsg -t get_tree`: every container that has a process.
fn sway_windows(json: &str) -> Vec<Window> {
    fn walk(node: &serde_json::Value, out: &mut Vec<Window>) {
        if let (Some(id), Some(pid)) = (node["id"].as_u64(), node["pid"].as_u64().and_then(|p| u32::try_from(p).ok())) {
            out.push(Window { id, pid });
        }
        for child in ["nodes", "floating_nodes"].iter().filter_map(|k| node[*k].as_array()).flatten() {
            walk(child, out);
        }
    }
    let mut out = Vec::new();
    if let Ok(tree) = serde_json::from_str::<serde_json::Value>(json) {
        walk(&tree, &mut out);
    }
    out
}

/// niri's `niri msg --json windows`.
fn niri_windows(json: &str) -> Vec<Window> {
    let windows: Vec<serde_json::Value> = serde_json::from_str(json).unwrap_or_default();
    windows.iter().filter_map(|w| Some(Window { id: w["id"].as_u64()?, pid: u32::try_from(w["pid"].as_u64()?).ok()? })).collect()
}

/// The client last active, from `tmux list-clients -F '#{client_activity} #{client_pid} #{client_tty}'`: its pid
/// and tty.
fn latest_client(list: &str) -> Option<(u32, String)> {
    list.lines()
        .filter_map(|line| {
            let mut fields = line.split_whitespace();
            let activity: u64 = fields.next()?.parse().ok()?;
            let pid: u32 = fields.next()?.parse().ok()?;
            Some((activity, pid, fields.next()?.to_string()))
        })
        .max_by_key(|(activity, ..)| *activity)
        .map(|(_, pid, tty)| (pid, tty))
}

/// The variables of process `pid` (Linux: from /proc; elsewhere none).
fn environ(pid: u32) -> HashMap<String, String> {
    let raw = std::fs::read(format!("/proc/{pid}/environ")).unwrap_or_default();
    raw.split(|b| *b == 0).filter_map(|entry| String::from_utf8_lossy(entry).split_once('=').map(|(k, v)| (k.to_string(), v.to_string()))).collect()
}

/// The parents of process `pid`, nearest first, up to init (Linux: from /proc; elsewhere none).
fn parents(pid: u32) -> Vec<u32> {
    let mut out = Vec::new();
    let mut at = pid;
    while out.len() < 64 {
        match std::fs::read_to_string(format!("/proc/{at}/stat")).ok().as_deref().and_then(parent_in) {
            Some(parent) if parent > 1 => {
                out.push(parent);
                at = parent;
            }
            _ => break,
        }
    }
    out
}

/// The parent's pid in a `/proc/<pid>/stat` line.
fn parent_in(stat: &str) -> Option<u32> {
    // After the name, which may hold spaces and brackets: the state, then the parent.
    stat[stat.rfind(')')? + 1..].split_whitespace().nth(1)?.parse().ok()
}

/// The script id in `dbus-send --print-reply`'s answer to `loadScript` (none when KWin refused it: -1).
fn kwin_script_id(reply: &str) -> Option<i32> {
    let mut words = reply.split_whitespace();
    words.find(|w| *w == "int32")?;
    words.next()?.parse().ok().filter(|id| *id >= 0)
}

/// The KWin script activating the window of the first of `pids` that has one (Plasma 6's API, else 5's).
fn kwin_script(pids: &[u32]) -> String {
    let pids = pids.iter().map(u32::to_string).collect::<Vec<_>>().join(", ");
    format!(
        "var pids = [{pids}];\n\
         var windows = workspace.windowList ? workspace.windowList() : workspace.clientList();\n\
         var found = null;\n\
         for (var i = 0; i < pids.length && !found; i++) {{\n\
         \x20   for (var j = 0; j < windows.length; j++) {{\n\
         \x20       if (windows[j].pid === pids[i]) {{ found = windows[j]; break; }}\n\
         \x20   }}\n\
         }}\n\
         if (found) {{ if (workspace.windowList) {{ workspace.activeWindow = found; }} else {{ workspace.activeClient = found; }} }}\n"
    )
}

/// Load, run and unload the KWin script for `pids` over D-Bus (`dbus-send`).
fn kwin(pids: &[u32], plasma6: bool) {
    use std::io::Write;
    let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().subsec_nanos();
    let path = std::env::temp_dir().join(format!("ttyak-raise-{}-{nanos}.js", std::process::id()));
    // (A new file: never one someone put there.)
    let written = std::fs::OpenOptions::new().write(true).create_new(true).open(&path).and_then(|mut f| f.write_all(kwin_script(pids).as_bytes()));
    if let Err(e) = written {
        tracing::debug!("raise: the KWin script: {e}");
        return;
    }
    let call = |object: &str, method: &str, args: &[String]| {
        let mut command = argv(&["dbus-send", "--session", "--print-reply", "--dest=org.kde.KWin", object, method]);
        command.extend_from_slice(args);
        output(&command, None)
    };
    let unload = || call("/Scripting", "org.kde.kwin.Scripting.unloadScript", &[format!("string:{KWIN_SCRIPT}")]);
    // One left by an earlier try would make KWin refuse the name.
    unload();
    let loaded = call("/Scripting", "org.kde.kwin.Scripting.loadScript", &[format!("string:{}", path.display()), format!("string:{KWIN_SCRIPT}")]);
    match loaded.as_deref().and_then(kwin_script_id) {
        Some(id) => {
            let object = if plasma6 { format!("/Scripting/Script{id}") } else { format!("/{id}") };
            call(&object, "org.kde.kwin.Script.run", &[]);
        }
        None => tracing::debug!("raise: KWin did not load the script"),
    }
    unload();
    let _ = std::fs::remove_file(&path);
}

/// The X server's client windows (`_NET_CLIENT_LIST`) with their `_NET_WM_PID`.
#[cfg(target_os = "linux")]
fn x11_windows() -> Vec<Window> {
    use x11rb::connection::Connection;
    use x11rb::protocol::xproto::{AtomEnum, ConnectionExt};
    let found = bounded(|| {
        let (conn, screen) = x11rb::connect(None).ok()?;
        let root = conn.setup().roots.get(screen)?.root;
        let atom = |name: &[u8]| conn.intern_atom(false, name).ok()?.reply().ok().map(|r| r.atom);
        let (list, pid) = (atom(b"_NET_CLIENT_LIST")?, atom(b"_NET_WM_PID")?);
        let windows: Vec<u32> = conn.get_property(false, root, list, AtomEnum::WINDOW, 0, u32::MAX).ok()?.reply().ok()?.value32()?.collect();
        let owner = |w: u32| conn.get_property(false, w, pid, AtomEnum::CARDINAL, 0, 1).ok()?.reply().ok()?.value32()?.next();
        Some(windows.into_iter().filter_map(|w| Some(Window { id: w.into(), pid: owner(w)? })).collect::<Vec<_>>())
    });
    found.unwrap_or_default()
}

#[cfg(not(target_os = "linux"))]
fn x11_windows() -> Vec<Window> {
    Vec::new()
}

/// Ask the window manager to activate `window` (EWMH `_NET_ACTIVE_WINDOW`, source 2: on the user's behalf).
#[cfg(target_os = "linux")]
fn x11_activate(window: u64) {
    use x11rb::connection::Connection;
    use x11rb::protocol::xproto::{ClientMessageEvent, ConnectionExt, EventMask};
    let Ok(window) = u32::try_from(window) else { return };
    let sent = bounded(move || {
        let (conn, screen) = x11rb::connect(None).ok()?;
        let root = conn.setup().roots.get(screen)?.root;
        let active = conn.intern_atom(false, b"_NET_ACTIVE_WINDOW").ok()?.reply().ok()?.atom;
        let event = ClientMessageEvent::new(32, window, active, [2, x11rb::CURRENT_TIME, 0, 0, 0]);
        conn.send_event(false, root, EventMask::SUBSTRUCTURE_REDIRECT | EventMask::SUBSTRUCTURE_NOTIFY, event).ok()?;
        conn.flush().ok()
    });
    if sent.is_none() {
        tracing::debug!("raise: the X server took no _NET_ACTIVE_WINDOW");
    }
}

#[cfg(not(target_os = "linux"))]
fn x11_activate(_window: u64) {}

#[cfg(test)]
mod tests {
    use super::*;

    const KITTY_TO: &str = "unix:/tmp/kitty-100.sock";

    /// An environment of `vars`, parents `ancestors` (nearest first) and windows `(id, pid)`, on Linux.
    fn at(vars: &[(&str, &str)], ancestors: &[u32], windows: &[(u64, u32)]) -> Env {
        Env {
            vars: vars.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect(),
            ancestors: ancestors.to_vec(),
            os: "linux",
            windows: windows.iter().map(|&(id, pid)| Window { id, pid }).collect(),
            ..Env::default()
        }
    }

    fn run(argv: &[&str]) -> Step {
        Step::Run(argv.iter().map(|a| a.to_string()).collect(), None)
    }

    /// kitty (pid 100, its shell 300) on Hyprland, another app's window as well.
    fn kitty_on_hyprland(answers: bool) -> Env {
        let vars = [("KITTY_WINDOW_ID", "5"), ("KITTY_LISTEN_ON", KITTY_TO), ("HYPRLAND_INSTANCE_SIGNATURE", "abc"), ("WAYLAND_DISPLAY", "wayland-1"), ("DISPLAY", ":1"), ("XDG_CURRENT_DESKTOP", "Hyprland")];
        Env { kitty_answers: answers, ..at(&vars, &[300, 100, 50], &[(0x5a, 999), (0x6b, 100)]) }
    }

    #[test]
    fn kitty_picks_the_tab_and_hyprland_the_workspace() {
        let hyprland = Raise::Hyprland { pid: 100 };
        let kitty = Raise::Kitty { window: "5".into(), to: KITTY_TO.into(), then: Box::new(hyprland.clone()) };
        assert_eq!(detect(&kitty_on_hyprland(true)), kitty);
        assert_eq!(steps(&kitty), vec![run(&["kitty", "@", "--to", KITTY_TO, "focus-window", "--match", "id:5"]), run(&["hyprctl", "dispatch", "focuswindow", "pid:100"])]);
        assert_eq!(detect(&kitty_on_hyprland(false)), hyprland, "remote control off: the window manager alone");
        let no_socket = Env { vars: kitty_on_hyprland(true).vars.into_iter().filter(|(k, _)| k != "KITTY_LISTEN_ON").collect(), ..kitty_on_hyprland(true) };
        assert_eq!(detect(&no_socket), hyprland, "never through the terminal's own tty, which ttyak reads");
        assert_eq!(detect(&at(&[("HYPRLAND_INSTANCE_SIGNATURE", "abc")], &[300, 100], &[(0x5a, 999)])), Raise::None, "no parent of ttyak has a window");
        assert_eq!(describe(&kitty), "via kitty and Hyprland");
        assert_eq!(describe(&hyprland), "via Hyprland");
    }

    #[test]
    fn tmux_switches_the_client_then_raises_the_outer_terminal() {
        let outer = kitty_on_hyprland(true);
        let inside = Env { tmux_client: Some(("/dev/pts/4".into(), Box::new(outer))), ..at(&[("TMUX", "/tmp/tmux-1000/default,77,0"), ("TMUX_PANE", "%3"), ("HYPRLAND_INSTANCE_SIGNATURE", "abc")], &[400, 77], &[]) };
        let tmux = detect(&inside);
        let kitty = Raise::Kitty { window: "5".into(), to: KITTY_TO.into(), then: Box::new(Raise::Hyprland { pid: 100 }) };
        assert_eq!(tmux, Raise::Tmux { pane: "%3".into(), client: "/dev/pts/4".into(), then: Box::new(kitty) });
        assert_eq!(
            steps(&tmux)[..3],
            [run(&["tmux", "select-window", "-t", "%3"]), run(&["tmux", "select-pane", "-t", "%3"]), run(&["tmux", "switch-client", "-c", "/dev/pts/4", "-t", "%3"])]
        );
        assert_eq!(steps(&tmux).len(), 5, "then kitty and Hyprland");
        assert_eq!(describe(&tmux), "via tmux, kitty and Hyprland");
        let detached = Env { tmux_client: None, ..inside };
        assert_eq!(detect(&detached), Raise::None, "no client: nothing on a screen to raise");
    }

    #[test]
    fn sway_and_niri_by_the_terminals_process() {
        let sway = at(&[("SWAYSOCK", "/run/user/1000/sway-ipc.sock"), ("WAYLAND_DISPLAY", "wayland-1")], &[300, 200], &[(7, 200)]);
        assert_eq!(detect(&sway), Raise::Sway { pid: 200 });
        assert_eq!(steps(&Raise::Sway { pid: 200 }), vec![run(&["swaymsg", "[pid=200] focus"])]);
        let niri = at(&[("NIRI_SOCKET", "/run/user/1000/niri.sock"), ("WAYLAND_DISPLAY", "wayland-1")], &[300, 200], &[(9, 111), (12, 200)]);
        assert_eq!(detect(&niri), Raise::Niri { window: 12 });
        assert_eq!(steps(&Raise::Niri { window: 12 }), vec![run(&["niri", "msg", "action", "focus-window", "--id", "12"])]);
        assert_eq!(describe(&Raise::Sway { pid: 200 }), "via sway");
    }

    #[test]
    fn wezterm_activates_the_pane() {
        let env = at(&[("WEZTERM_PANE", "2"), ("WEZTERM_UNIX_SOCKET", "/run/user/1000/wezterm/gui-sock-9"), ("SWAYSOCK", "/s")], &[300, 9], &[(4, 9)]);
        let r = detect(&env);
        assert_eq!(r, Raise::Wezterm { pane: "2".into(), socket: Some("/run/user/1000/wezterm/gui-sock-9".into()), then: Box::new(Raise::Sway { pid: 9 }) });
        assert_eq!(steps(&r)[0], Step::Run(["wezterm", "cli", "activate-pane", "--pane-id", "2"].map(String::from).to_vec(), Some(("WEZTERM_UNIX_SOCKET", "/run/user/1000/wezterm/gui-sock-9".into()))));
    }

    #[test]
    fn x11_by_windowid_or_the_window_of_a_parent() {
        assert_eq!(detect(&at(&[("DISPLAY", ":0"), ("WINDOWID", "41943047")], &[300, 100], &[])), Raise::X11 { window: 41943047 });
        assert_eq!(detect(&at(&[("DISPLAY", ":0")], &[300, 100], &[(0x1c00007, 100)])), Raise::X11 { window: 0x1c00007 });
        assert_eq!(steps(&Raise::X11 { window: 5 }), vec![Step::X11(5)]);
        assert_eq!(detect(&at(&[("DISPLAY", ":0"), ("WAYLAND_DISPLAY", "wayland-0"), ("WINDOWID", "41943047")], &[300], &[])), Raise::None, "XWayland's DISPLAY is no X11 desktop");
    }

    #[test]
    fn kde_through_a_kwin_script() {
        let r = detect(&at(&[("KDE_SESSION_VERSION", "6"), ("WAYLAND_DISPLAY", "wayland-0"), ("XDG_CURRENT_DESKTOP", "KDE")], &[300, 100], &[]));
        assert_eq!(r, Raise::Kwin { pids: vec![300, 100], plasma6: true });
        assert_eq!(steps(&r), vec![Step::Kwin(vec![300, 100], true)]);
        assert_eq!(detect(&at(&[("KDE_SESSION_VERSION", "5"), ("DISPLAY", ":0")], &[300], &[])), Raise::Kwin { pids: vec![300], plasma6: false });
        let script = kwin_script(&[300, 100]);
        assert!(script.contains("[300, 100]") && script.contains("workspace.activeWindow") && script.contains("workspace.activeClient"), "{script}");
        assert_eq!(kwin_script_id("method return time=1.2 sender=:1.5 -> destination=:1.9 serial=7 reply_serial=2\n   int32 3\n"), Some(3));
        assert_eq!(kwin_script_id("method return time=1.2\n   int32 -1\n"), None);
        assert_eq!(describe(&r), "via KWin");
    }

    #[test]
    fn gnome_on_wayland_cannot() {
        let gnome = at(&[("XDG_CURRENT_DESKTOP", "GNOME"), ("WAYLAND_DISPLAY", "wayland-0"), ("DISPLAY", ":0")], &[300, 100], &[]);
        assert_eq!(detect(&gnome), Raise::None);
        assert!(steps(&Raise::None).is_empty());
        assert_eq!(describe(&Raise::None), CANNOT);
        let mac = Env { os: "macos", ..at(&[("DISPLAY", ":0"), ("WINDOWID", "7")], &[300], &[]) };
        assert_eq!(detect(&mac), Raise::None, "macOS and Windows have routes of their own");
    }

    #[test]
    fn what_the_tools_say() {
        // Shapes as the tools print them (Hyprland 0.56's captured here, trimmed; sway's and niri's from their IPC docs).
        let hyprland = r#"[{"address": "0x56c1d9bfba40", "mapped": true, "pid": 625833, "class": "kitty"}, {"address": "0x56c1d9e331b0", "pid": 1545088}]"#;
        assert_eq!(hyprland_windows(hyprland), vec![Window { id: 0x56c1d9bfba40, pid: 625833 }, Window { id: 0x56c1d9e331b0, pid: 1545088 }]);
        let sway = r#"{"id": 1, "type": "root", "nodes": [{"id": 3, "type": "output", "nodes": [{"id": 4, "type": "workspace", "nodes": [{"id": 6, "type": "con", "pid": 812, "nodes": []}], "floating_nodes": [{"id": 8, "type": "floating_con", "pid": 813, "nodes": []}]}]}]}"#;
        assert_eq!(sway_windows(sway), vec![Window { id: 6, pid: 812 }, Window { id: 8, pid: 813 }]);
        let niri = r#"[{"id": 12, "title": "x", "app_id": "kitty", "pid": 200, "workspace_id": 1, "is_focused": false}, {"id": 13, "pid": null}]"#;
        assert_eq!(niri_windows(niri), vec![Window { id: 12, pid: 200 }]);
        assert!(hyprland_windows("not json").is_empty());
        assert_eq!(latest_client("1792900000 4100 /dev/pts/3\n1792900100 4200 /dev/pts/5\n1792899999 4300 /dev/pts/6\n"), Some((4200, "/dev/pts/5".into())));
        assert_eq!(latest_client(""), None);
        assert_eq!(parent_in("1545088 (kitty) S 1545082 1545082 1545082 0 -1"), Some(1545082));
        assert_eq!(parent_in("77 (tmux: server) (x) S 1 77 77 0"), Some(1), "a name with spaces and brackets");
    }

    /// What this machine picks (read-only: the tools are only asked, nothing is raised).
    #[test]
    #[ignore]
    fn live_detect() {
        let r = detect(&env());
        println!("{r:?} -> {}", describe(&r));
    }

    /// Brings the window forward by what is picked — only with `TTYAK_RAISE_LIVE=1`, where it can't move the user's
    /// focus (a headless compositor in a container).
    #[test]
    #[ignore]
    fn live_raise() {
        if std::env::var("TTYAK_RAISE_LIVE").as_deref() != Ok("1") {
            return;
        }
        let r = detect(&env());
        println!("{r:?} -> {}", describe(&r));
        raise(&r);
    }
}
