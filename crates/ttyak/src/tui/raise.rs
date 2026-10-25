//! Bringing the terminal's window forward after a click on the system notifier's bubble (the terminal's own bubbles,
//! OSC 99, bring it forward themselves): tmux's window, pane and client, the terminal's remote control (kitty, WezTerm),
//! the window manager (Hyprland, sway, niri, KDE's KWin, X11). Where nothing can (GNOME on Wayland), nothing happens.

use std::collections::HashMap;
use std::io::Read;
use std::time::{Duration, Instant};

/// How long one command (or the X server) may take before it is given up (internal).
const COMMAND_WAIT: Duration = Duration::from_secs(2);

/// How long the terminal may take to retitle its window after it focused ttyak's tab, and how often it is looked at
/// meanwhile (internal).
const RETITLE_WAIT: Duration = Duration::from_secs(1);
const RETITLE_POLL: Duration = Duration::from_millis(100);

/// The name the KWin script is loaded under.
const KWIN_SCRIPT: &str = "ttyak-raise";

/// The variables the desktop's tools find it by. They are taken from the environment the window was found in: inside
/// tmux the outer terminal's, since the tmux server's may be stale (re-attached after logging in again, or after the
/// compositor restarted).
const DESKTOP_VARS: &[&str] = &["HYPRLAND_INSTANCE_SIGNATURE", "XDG_RUNTIME_DIR", "SWAYSOCK", "NIRI_SOCKET", "DISPLAY", "XAUTHORITY", "WAYLAND_DISPLAY", "DBUS_SESSION_BUS_ADDRESS"];

/// [`describe`]'s words (through `tr`).
pub const VIA: &str = "via {routes}";
pub const AND: &str = "{first} and {last}";
pub const COMMA: &str = ", ";
pub const CANNOT: &str = "the window can't be raised on this desktop";
pub const WORDS: &[&str] = &[VIA, AND, COMMA, CANNOT];

/// A window the window manager lists: its id, the process it belongs to, its title.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Window {
    pub id: u64,
    pub pid: u32,
    pub title: String,
}

/// kitty's answer about ttyak's window (`KITTY_WINDOW_ID`): the process it runs and its title.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KittyWindow {
    pub pid: u32,
    pub title: String,
}

/// What the raise is picked from: ttyak's environment variables, its parent processes (nearest first), the OS, and
/// what the tools the variables point at answered ([`env`] asks them).
#[derive(Clone, Debug, Default)]
pub struct Env {
    pub vars: HashMap<String, String>,
    pub ancestors: Vec<u32>,
    pub os: &'static str,
    /// The process itself (for the outer terminal of tmux: the tmux client).
    pub pid: u32,
    /// What ttyak last put into the terminal's title, if anything (tells apart the windows of one terminal process).
    pub title: Option<String>,
    /// kitty's answer to `kitty @ ls` at `KITTY_LISTEN_ON` (remote control is on).
    pub kitty: Option<KittyWindow>,
    /// The windows of the window manager the variables point at (Hyprland's clients, sway's tree, niri's windows, the X
    /// server's `_NET_CLIENT_LIST`).
    pub windows: Vec<Window>,
    /// Inside tmux: the client last active on ttyak's session — its tty, and the outer terminal as that client sees it
    /// (its variables and parents).
    pub tmux_client: Option<(String, Box<Env>)>,
}

/// Which of the window manager's windows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Pick {
    Id(u64),
    /// The one window of `pid` titled `title`, looked up when raising: the terminal retitles its window once it has
    /// focused ttyak's tab.
    Titled { pid: u32, title: String },
}

/// How the window is brought forward.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Raise {
    /// tmux: ttyak's window and pane selected, the client (its tty) switched to them; then the outer terminal, whose
    /// tools are run with its desktop's variables (`vars`).
    Tmux { pane: String, client: String, vars: Vec<(String, String)>, then: Box<Raise> },
    /// kitty's remote control at `to`: ttyak's window and tab focused; then the window manager.
    Kitty { window: String, to: String, then: Box<Raise> },
    /// WezTerm's: ttyak's pane activated (through `socket`, when known); then the window manager.
    Wezterm { pane: String, socket: Option<String>, then: Box<Raise> },
    Hyprland { window: Pick },
    Sway { window: Pick },
    Niri { window: Pick },
    /// KDE: a KWin script activates the window of the first of `pids` that has one (with several, the one titled
    /// `title`); Plasma 6, else 5.
    Kwin { pids: Vec<u32>, title: Option<String>, plasma6: bool },
    /// EWMH `_NET_ACTIVE_WINDOW`.
    X11 { window: Pick },
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
    /// A command, run without a shell, with these variables set.
    Run(Vec<String>, Vec<(String, String)>),
    /// The window manager focuses this window (a titled one is looked up first), its tools run with these variables.
    Focus(Desktop, Pick, Vec<(String, String)>),
    /// The KWin script.
    Kwin { pids: Vec<u32>, title: Option<String>, plasma6: bool, vars: Vec<(String, String)> },
}

/// This process's environment, parents and OS, with the answers of the tools they point at (each asked for at most
/// [`COMMAND_WAIT`]).
pub fn env() -> Env {
    let (vars, ancestors): (HashMap<String, String>, _) = (std::env::vars().collect(), parents(std::process::id()));
    if let (Some(_), Some(pane)) = (var(&vars, "TMUX"), var(&vars, "TMUX_PANE")) {
        // Inside tmux only the outer terminal is asked (the pane's variables are the tmux server's, maybe stale).
        let tmux_client = tmux_client(pane).map(|(pid, tty)| (tty, Box::new(probed(environ(pid), parents(pid), pid))));
        return Env { vars, ancestors, os: std::env::consts::OS, pid: std::process::id(), tmux_client, ..Env::default() };
    }
    probed(vars, ancestors, std::process::id())
}

/// The best raise for `env`, in the order: tmux, kitty, WezTerm, then the window manager.
pub fn detect(env: &Env) -> Raise {
    let var = |name: &str| var(&env.vars, name).map(String::from);
    if let (Some(_), Some(pane)) = (var("TMUX"), var("TMUX_PANE")) {
        // Without a client the session is on no screen.
        return match &env.tmux_client {
            Some((client, outer)) => Raise::Tmux { pane, client: client.clone(), vars: desktop_vars(&outer.vars), then: Box::new(detect(outer)) },
            None => Raise::None,
        };
    }
    // (Never through the terminal's own tty: ttyak reads it. A kitty window that runs none of ttyak's parents is one
    // whose variables were inherited, e.g. by another terminal started from it.)
    let kitty = env.kitty.as_ref().filter(|k| k.pid == env.pid || env.ancestors.contains(&k.pid));
    if let (Some(window), Some(to), Some(kitty)) = (var("KITTY_WINDOW_ID"), var("KITTY_LISTEN_ON"), kitty) {
        return Raise::Kitty { window, to, then: Box::new(window_manager(env, Some(&kitty.title), true)) };
    }
    if let Some(pane) = var("WEZTERM_PANE") {
        return Raise::Wezterm { pane, socket: var("WEZTERM_UNIX_SOCKET"), then: Box::new(window_manager(env, env.title.as_deref(), true)) };
    }
    window_manager(env, env.title.as_deref(), false)
}

/// The window manager's raise, of the nearest parent of ttyak that has a window. `title`: the terminal window's, to
/// tell apart several windows of one process; `retitled`: a step before this one makes it the window's title.
fn window_manager(env: &Env, title: Option<&str>, retitled: bool) -> Raise {
    let pick = || pick(env, title, retitled);
    match desktop(&env.vars, env.os) {
        Desktop::Hyprland => pick().map_or(Raise::None, |window| Raise::Hyprland { window }),
        Desktop::Sway => pick().map_or(Raise::None, |window| Raise::Sway { window }),
        Desktop::Niri => pick().map_or(Raise::None, |window| Raise::Niri { window }),
        // (The script picks the window itself: by process, then by title.)
        Desktop::Kde { .. } if env.ancestors.is_empty() => Raise::None,
        Desktop::Kde { plasma6 } => Raise::Kwin { pids: env.ancestors.clone(), title: title.map(String::from), plasma6 },
        Desktop::X11 => var(&env.vars, "WINDOWID").and_then(|w| w.parse().ok()).map(Pick::Id).or_else(pick).map_or(Raise::None, |window| Raise::X11 { window }),
        Desktop::Other => Raise::None,
    }
}

/// The window of the nearest parent that has one; of several, the one titled `title` — or, when `retitled`, the one
/// that will be. None when that can't be told (rather than raising the wrong one).
fn pick(env: &Env, title: Option<&str>, retitled: bool) -> Option<Pick> {
    let pid = *env.ancestors.iter().find(|pid| env.windows.iter().any(|w| w.pid == **pid))?;
    let of: Vec<&Window> = env.windows.iter().filter(|w| w.pid == pid).collect();
    if let [one] = of[..] {
        return Some(Pick::Id(one.id));
    }
    let title = title?;
    match titled(&env.windows, pid, title) {
        Some(id) => Some(Pick::Id(id)),
        None if retitled => Some(Pick::Titled { pid, title: title.to_string() }),
        None => None,
    }
}

/// The one window of `pid` titled `title`.
fn titled(windows: &[Window], pid: u32, title: &str) -> Option<u64> {
    match windows.iter().filter(|w| w.pid == pid && w.title == title).collect::<Vec<_>>()[..] {
        [one] => Some(one.id),
        _ => None,
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

/// [`DESKTOP_VARS`] of `vars` (without `XAUTHORITY`, X's default file in their `HOME`).
fn desktop_vars(vars: &HashMap<String, String>) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = DESKTOP_VARS.iter().filter_map(|name| var(vars, name).map(|v| (name.to_string(), v.to_string()))).collect();
    if var(vars, "XAUTHORITY").is_none() {
        if let Some(home) = var(vars, "HOME") {
            out.push(("XAUTHORITY".into(), format!("{home}/.Xauthority")));
        }
    }
    out
}

/// Variable `name` of a [`desktop_vars`] list.
#[cfg(any(target_os = "linux", test))]
fn get<'a>(vars: &'a [(String, String)], name: &str) -> Option<&'a str> {
    vars.iter().find(|(k, _)| k == name).map(|(_, v)| v.as_str())
}

/// An environment of `vars`, `ancestors` and process `pid`, with what the tools they point at answer.
fn probed(vars: HashMap<String, String>, ancestors: Vec<u32>, pid: u32) -> Env {
    let os = std::env::consts::OS;
    let tools = desktop_vars(&vars);
    let kitty = match (var(&vars, "KITTY_WINDOW_ID"), var(&vars, "KITTY_LISTEN_ON")) {
        (Some(window), Some(to)) => output(&argv(&["kitty", "@", "--to", to, "ls", "--match", &format!("id:{window}")]), &tools).and_then(|ls| kitty_window(&ls, window)),
        _ => None,
    };
    let windows = listed(&vars, os).map(|desktop| windows_of(desktop, &tools)).unwrap_or_default();
    Env { vars, ancestors, os, pid, title: None, kitty, windows, tmux_client: None }
}

/// The desktop whose windows are worth listing for `vars`: not X11 with `WINDOWID` (exact already), nor over ssh -X,
/// where ttyak's parents are the remote machine's processes.
fn listed(vars: &HashMap<String, String>, os: &str) -> Option<Desktop> {
    match desktop(vars, os) {
        Desktop::X11 if var(vars, "WINDOWID").is_some() || var(vars, "SSH_CONNECTION").is_some() => None,
        Desktop::Kde { .. } | Desktop::Other => None,
        desktop => Some(desktop),
    }
}

/// The windows `desktop` lists (its tools run with `vars`).
fn windows_of(desktop: Desktop, vars: &[(String, String)]) -> Vec<Window> {
    let listed = |command: &[&str], parse: fn(&str) -> Vec<Window>| output(&argv(command), vars).map(|out| parse(&out)).unwrap_or_default();
    match desktop {
        Desktop::Hyprland => listed(&["hyprctl", "-j", "clients"], hyprland_windows),
        Desktop::Sway => listed(&["swaymsg", "-r", "-t", "get_tree"], sway_windows),
        Desktop::Niri => listed(&["niri", "msg", "--json", "windows"], niri_windows),
        Desktop::X11 => x11_windows(vars),
        Desktop::Kde { .. } | Desktop::Other => Vec::new(),
    }
}

/// The tmux client last active on the session of `pane`: its pid and tty.
fn tmux_client(pane: &str) -> Option<(u32, String)> {
    let session = output(&argv(&["tmux", "display-message", "-p", "-t", pane, "#{session_id}"]), &[])?;
    let list = output(&argv(&["tmux", "list-clients", "-t", session.trim(), "-F", "#{client_activity} #{client_pid} #{client_tty}"]), &[])?;
    latest_client(&list)
}

/// Bring the window forward by `r`; each command at most [`COMMAND_WAIT`], every step logged.
pub fn raise(r: &Raise) {
    for step in steps(r, &[]) {
        match step {
            Step::Run(argv, vars) => drop(output(&argv, &vars)),
            Step::Focus(desktop, pick, vars) => {
                let id = match pick {
                    Pick::Id(id) => Some(id),
                    Pick::Titled { pid, title } => retitled(desktop, pid, &title, &vars),
                };
                match (id, desktop) {
                    (None, _) => tracing::debug!(?desktop, "raise: the terminal's window can't be told apart from its others: left"),
                    (Some(id), Desktop::X11) => x11_activate(id, &vars),
                    (Some(id), _) => drop(output(&focus_command(desktop, id), &vars)),
                }
            }
            Step::Kwin { pids, title, plasma6, vars } => kwin(&pids, title.as_deref(), plasma6, &vars),
        }
    }
}

/// The window of `pid` titled `title`, once the terminal has retitled it (at most [`RETITLE_WAIT`]).
fn retitled(desktop: Desktop, pid: u32, title: &str, vars: &[(String, String)]) -> Option<u64> {
    let until = Instant::now() + RETITLE_WAIT;
    loop {
        if let Some(id) = titled(&windows_of(desktop, vars), pid, title) {
            return Some(id);
        }
        if Instant::now() >= until {
            return None;
        }
        std::thread::sleep(RETITLE_POLL);
    }
}

/// Bring the window forward in the background (the tools are asked anew each time: windows and tmux clients come and
/// go), never holding up the caller. `title`: what ttyak last put into the terminal's title.
pub fn bring_forward(title: Option<String>) {
    std::thread::spawn(move || {
        let r = detect(&Env { title, ..env() });
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
        Some((last, first)) => tr(AND).replace("{first}", &first.join(&tr(COMMA))).replace("{last}", last),
    };
    tr(VIA).replace("{routes}", &routes)
}

/// What runs for `r`, in order; the window manager's tools with `vars` (none: ttyak's own environment).
fn steps(r: &Raise, vars: &[(String, String)]) -> Vec<Step> {
    let run = |args: &[&str]| Step::Run(argv(args), vars.to_vec());
    match r {
        Raise::Tmux { pane, client, vars, then } => {
            // One invocation (`;` separates tmux's commands; no shell), with ttyak's own environment: its tmux server.
            let select = argv(&["tmux", "select-window", "-t", pane, ";", "select-pane", "-t", pane, ";", "switch-client", "-c", client, "-t", pane]);
            [vec![Step::Run(select, Vec::new())], steps(then, vars)].concat()
        }
        Raise::Kitty { window, to, then } => [vec![run(&["kitty", "@", "--to", to, "focus-window", "--match", &format!("id:{window}")])], steps(then, vars)].concat(),
        Raise::Wezterm { pane, socket, then } => {
            let mut with = vars.to_vec();
            with.extend(socket.clone().map(|s| ("WEZTERM_UNIX_SOCKET".to_string(), s)));
            [vec![Step::Run(argv(&["wezterm", "cli", "activate-pane", "--pane-id", pane]), with)], steps(then, vars)].concat()
        }
        Raise::Hyprland { window } => vec![Step::Focus(Desktop::Hyprland, window.clone(), vars.to_vec())],
        Raise::Sway { window } => vec![Step::Focus(Desktop::Sway, window.clone(), vars.to_vec())],
        Raise::Niri { window } => vec![Step::Focus(Desktop::Niri, window.clone(), vars.to_vec())],
        Raise::X11 { window } => vec![Step::Focus(Desktop::X11, window.clone(), vars.to_vec())],
        Raise::Kwin { pids, title, plasma6 } => vec![Step::Kwin { pids: pids.clone(), title: title.clone(), plasma6: *plasma6, vars: vars.to_vec() }],
        Raise::None => Vec::new(),
    }
}

/// The command focusing window `id` on `desktop` (X11 has none: [`x11_activate`]).
fn focus_command(desktop: Desktop, id: u64) -> Vec<String> {
    match desktop {
        Desktop::Hyprland => argv(&["hyprctl", "dispatch", "focuswindow", &format!("address:0x{id:x}")]),
        Desktop::Sway => argv(&["swaymsg", &format!("[con_id={id}] focus")]),
        Desktop::Niri => argv(&["niri", "msg", "action", "focus-window", "--id", &id.to_string()]),
        Desktop::Kde { .. } | Desktop::X11 | Desktop::Other => Vec::new(),
    }
}

fn argv(args: &[&str]) -> Vec<String> {
    args.iter().map(|a| a.to_string()).collect()
}

/// Run `argv` without a shell, with `vars` set, for at most [`COMMAND_WAIT`]: what it printed, when it succeeded.
/// Every run is logged (debug) with how it ended.
fn output(argv: &[String], vars: &[(String, String)]) -> Option<String> {
    use std::process::{Command, Stdio};
    let (program, args) = argv.split_first()?;
    let line = argv.join(" ");
    let mut command = Command::new(program);
    command.args(args).envs(vars.iter().map(|(k, v)| (k, v))).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::null());
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(e) => {
            tracing::debug!(command = line, "raise: {e}");
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
            Ok(Some(status)) => {
                tracing::debug!(command = line, %status, "raise");
                return status.success().then_some(text).flatten();
            }
            Ok(None) if text.is_some() && Instant::now() < deadline => std::thread::sleep(Duration::from_millis(10)),
            _ => {
                tracing::debug!(command = line, "raise: no answer within {} s", COMMAND_WAIT.as_secs());
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

/// kitty's window `id` in `kitty @ ls`'s answer.
fn kitty_window(ls: &str, id: &str) -> Option<KittyWindow> {
    let os_windows: Vec<serde_json::Value> = serde_json::from_str(ls).ok()?;
    let windows = os_windows.iter().flat_map(|o| o["tabs"].as_array().into_iter().flatten()).flat_map(|t| t["windows"].as_array().into_iter().flatten());
    let window = windows.into_iter().find(|w| w["id"].as_u64().is_some_and(|n| n.to_string() == id))?;
    Some(KittyWindow { pid: u32::try_from(window["pid"].as_u64()?).ok()?, title: window["title"].as_str().unwrap_or_default().to_string() })
}

/// Hyprland's `hyprctl -j clients`.
fn hyprland_windows(json: &str) -> Vec<Window> {
    let clients: Vec<serde_json::Value> = serde_json::from_str(json).unwrap_or_default();
    clients
        .iter()
        .filter_map(|c| {
            let id = u64::from_str_radix(c["address"].as_str()?.trim_start_matches("0x"), 16).ok()?;
            Some(Window { id, pid: u32::try_from(c["pid"].as_u64()?).ok()?, title: c["title"].as_str().unwrap_or_default().to_string() })
        })
        .collect()
}

/// sway's `swaymsg -t get_tree`: every container that has a process.
fn sway_windows(json: &str) -> Vec<Window> {
    fn walk(node: &serde_json::Value, out: &mut Vec<Window>) {
        if let (Some(id), Some(pid)) = (node["id"].as_u64(), node["pid"].as_u64().and_then(|p| u32::try_from(p).ok())) {
            out.push(Window { id, pid, title: node["name"].as_str().unwrap_or_default().to_string() });
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
    windows
        .iter()
        .filter_map(|w| Some(Window { id: w["id"].as_u64()?, pid: u32::try_from(w["pid"].as_u64()?).ok()?, title: w["title"].as_str().unwrap_or_default().to_string() }))
        .collect()
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

/// The KWin script activating the window of the first of `pids` that has one — of several, the one captioned `title`,
/// else none (Plasma 6's API, else 5's).
fn kwin_script(pids: &[u32], title: Option<&str>) -> String {
    let pids = pids.iter().map(u32::to_string).collect::<Vec<_>>().join(", ");
    // (A JSON string is a JavaScript string.)
    let title = title.map_or("null".to_string(), |t| serde_json::Value::from(t).to_string());
    format!(
        "var pids = [{pids}];\n\
         var title = {title};\n\
         var windows = workspace.windowList ? workspace.windowList() : workspace.clientList();\n\
         var found = null;\n\
         for (var i = 0; i < pids.length; i++) {{\n\
         \x20   var of = windows.filter(function (w) {{ return w.pid === pids[i]; }});\n\
         \x20   if (of.length === 0) {{ continue; }}\n\
         \x20   if (of.length > 1) {{ of = of.filter(function (w) {{ return w.caption === title; }}); }}\n\
         \x20   if (of.length === 1) {{ found = of[0]; }}\n\
         \x20   break;\n\
         }}\n\
         if (found) {{ if (workspace.windowList) {{ workspace.activeWindow = found; }} else {{ workspace.activeClient = found; }} }}\n"
    )
}

/// Load, run and unload the KWin script for `pids` over D-Bus (`dbus-send`, with `vars`).
fn kwin(pids: &[u32], title: Option<&str>, plasma6: bool, vars: &[(String, String)]) {
    use std::io::Write;
    let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().subsec_nanos();
    let path = std::env::temp_dir().join(format!("ttyak-raise-{}-{nanos}.js", std::process::id()));
    // (A new file: never one someone put there.)
    let written = std::fs::OpenOptions::new().write(true).create_new(true).open(&path).and_then(|mut f| f.write_all(kwin_script(pids, title).as_bytes()));
    if let Err(e) = written {
        tracing::debug!("raise: the KWin script: {e}");
        return;
    }
    let call = |object: &str, method: &str, args: &[String]| {
        let mut command = argv(&["dbus-send", "--session", "--print-reply", "--dest=org.kde.KWin", object, method]);
        command.extend_from_slice(args);
        output(&command, vars)
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

/// The cookie for display number `display` reached at `address` (`family`: whether a protocol family number is the one
/// connected with), from an `.Xauthority` file's bytes.
#[cfg(any(target_os = "linux", test))]
fn xauth_cookie(file: &[u8], family: impl Fn(u16) -> bool, address: &[u8], display: u16) -> Option<(Vec<u8>, Vec<u8>)> {
    const WILD: u16 = 65535;
    fn field<'a>(file: &'a [u8], at: &mut usize) -> Option<&'a [u8]> {
        let n = usize::from(u16::from_be_bytes(file.get(*at..*at + 2)?.try_into().ok()?));
        let f = file.get(*at + 2..*at + 2 + n)?;
        *at += 2 + n;
        Some(f)
    }
    let number = display.to_string();
    let mut at = 0;
    while at < file.len() {
        let entry_family = u16::from_be_bytes(file.get(at..at + 2)?.try_into().ok()?);
        at += 2;
        let (entry_address, entry_number, name, data) = (field(file, &mut at)?, field(file, &mut at)?, field(file, &mut at)?, field(file, &mut at)?);
        let reaches = entry_family == WILD || family(WILD) || (family(entry_family) && entry_address == address);
        if reaches && (entry_number.is_empty() || entry_number == number.as_bytes()) && name == b"MIT-MAGIC-COOKIE-1" {
            return Some((name.to_vec(), data.to_vec()));
        }
    }
    None
}

/// A connection to the X server of `vars`' `DISPLAY`, with the cookie from their `XAUTHORITY` file (no `DISPLAY` in
/// them: ttyak's own environment's).
#[cfg(target_os = "linux")]
fn x11_connect(vars: &[(String, String)]) -> Option<(x11rb::rust_connection::RustConnection, usize)> {
    use x11rb::reexports::x11rb_protocol::{parse_display::parse_display, xauth::Family};
    use x11rb::rust_connection::{DefaultStream, RustConnection};
    let Some(display) = get(vars, "DISPLAY") else { return x11rb::connect(None).ok() };
    let display = parse_display(Some(display)).ok()?;
    let file = get(vars, "XAUTHORITY").and_then(|f| std::fs::read(f).ok()).unwrap_or_default();
    let screen = usize::from(display.screen);
    for addr in display.connect_instruction() {
        let Ok((stream, (family, address))) = DefaultStream::connect(&addr) else { continue };
        let (name, data) = xauth_cookie(&file, |f| Family::from(f) == family, &address, display.display).unwrap_or_default();
        return RustConnection::connect_to_stream_with_auth_info(stream, screen, name, data).ok().map(|c| (c, screen));
    }
    None
}

/// The X server's client windows (`_NET_CLIENT_LIST`) with their `_NET_WM_PID` and `_NET_WM_NAME`.
#[cfg(target_os = "linux")]
fn x11_windows(vars: &[(String, String)]) -> Vec<Window> {
    use x11rb::connection::Connection;
    use x11rb::protocol::xproto::{AtomEnum, ConnectionExt};
    let vars = vars.to_vec();
    let found = bounded(move || {
        let (conn, screen) = x11_connect(&vars)?;
        let root = conn.setup().roots.get(screen)?.root;
        let atom = |name: &[u8]| conn.intern_atom(false, name).ok()?.reply().ok().map(|r| r.atom);
        let (list, pid, name, utf8) = (atom(b"_NET_CLIENT_LIST")?, atom(b"_NET_WM_PID")?, atom(b"_NET_WM_NAME")?, atom(b"UTF8_STRING")?);
        let windows: Vec<u32> = conn.get_property(false, root, list, AtomEnum::WINDOW, 0, u32::MAX).ok()?.reply().ok()?.value32()?.collect();
        let owner = |w: u32| conn.get_property(false, w, pid, AtomEnum::CARDINAL, 0, 1).ok()?.reply().ok()?.value32()?.next();
        let title = |w: u32| conn.get_property(false, w, name, utf8, 0, 1024).ok().and_then(|c| c.reply().ok()).map(|r| String::from_utf8_lossy(&r.value).into_owned()).unwrap_or_default();
        Some(windows.into_iter().filter_map(|w| Some(Window { id: w.into(), pid: owner(w)?, title: title(w) })).collect::<Vec<_>>())
    });
    found.unwrap_or_default()
}

#[cfg(not(target_os = "linux"))]
fn x11_windows(_vars: &[(String, String)]) -> Vec<Window> {
    Vec::new()
}

/// Ask the window manager to activate `window` (EWMH `_NET_ACTIVE_WINDOW`, source 2: on the user's behalf).
#[cfg(target_os = "linux")]
fn x11_activate(window: u64, vars: &[(String, String)]) {
    use x11rb::connection::Connection;
    use x11rb::protocol::xproto::{ClientMessageEvent, ConnectionExt, EventMask};
    let Ok(window) = u32::try_from(window) else { return };
    let vars = vars.to_vec();
    let sent = bounded(move || {
        let (conn, screen) = x11_connect(&vars)?;
        let root = conn.setup().roots.get(screen)?.root;
        let active = conn.intern_atom(false, b"_NET_ACTIVE_WINDOW").ok()?.reply().ok()?.atom;
        let event = ClientMessageEvent::new(32, window, active, [2, x11rb::CURRENT_TIME, 0, 0, 0]);
        conn.send_event(false, root, EventMask::SUBSTRUCTURE_REDIRECT | EventMask::SUBSTRUCTURE_NOTIFY, event).ok()?;
        // A round trip: the server has handled the message before the connection closes (it may drop what it had not
        // read yet from a client that went away).
        conn.get_input_focus().ok()?.reply().ok().map(drop)
    });
    tracing::debug!(window, sent = sent.is_some(), "raise: _NET_ACTIVE_WINDOW");
}

#[cfg(not(target_os = "linux"))]
fn x11_activate(_window: u64, _vars: &[(String, String)]) {}

#[cfg(test)]
mod tests {
    use super::*;

    const KITTY_TO: &str = "unix:/tmp/kitty-100.sock";

    /// An environment of `vars`, parents `ancestors` (nearest first) and windows `(id, pid, title)`, on Linux, as
    /// process 500.
    fn at(vars: &[(&str, &str)], ancestors: &[u32], windows: &[(u64, u32, &str)]) -> Env {
        Env {
            vars: vars.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect(),
            ancestors: ancestors.to_vec(),
            os: "linux",
            pid: 500,
            windows: windows.iter().map(|&(id, pid, title)| Window { id, pid, title: title.into() }).collect(),
            ..Env::default()
        }
    }

    fn run(argv: &[&str]) -> Step {
        Step::Run(argv.iter().map(|a| a.to_string()).collect(), Vec::new())
    }

    /// kitty (pid 100, its shell 300 running ttyak's window "ttyak") on Hyprland, another app's window as well.
    fn kitty_on_hyprland(kitty: Option<KittyWindow>) -> Env {
        let vars = [("KITTY_WINDOW_ID", "5"), ("KITTY_LISTEN_ON", KITTY_TO), ("HYPRLAND_INSTANCE_SIGNATURE", "abc"), ("WAYLAND_DISPLAY", "wayland-1"), ("DISPLAY", ":1"), ("XDG_CURRENT_DESKTOP", "Hyprland")];
        Env { kitty, ..at(&vars, &[300, 100, 50], &[(0x5a, 999, "browser"), (0x6b, 100, "ttyak")]) }
    }

    fn shell_300() -> Option<KittyWindow> {
        Some(KittyWindow { pid: 300, title: "ttyak".into() })
    }

    #[test]
    fn kitty_picks_the_tab_and_hyprland_the_workspace() {
        let hyprland = Raise::Hyprland { window: Pick::Id(0x6b) };
        let kitty = Raise::Kitty { window: "5".into(), to: KITTY_TO.into(), then: Box::new(hyprland.clone()) };
        assert_eq!(detect(&kitty_on_hyprland(shell_300())), kitty);
        assert_eq!(steps(&kitty, &[]), vec![run(&["kitty", "@", "--to", KITTY_TO, "focus-window", "--match", "id:5"]), Step::Focus(Desktop::Hyprland, Pick::Id(0x6b), Vec::new())]);
        assert_eq!(focus_command(Desktop::Hyprland, 0x6b), ["hyprctl", "dispatch", "focuswindow", "address:0x6b"]);
        assert_eq!(detect(&kitty_on_hyprland(None)), hyprland, "remote control off: the window manager alone");
        let inherited = Some(KittyWindow { pid: 4242, title: "ttyak".into() });
        assert_eq!(detect(&kitty_on_hyprland(inherited)), hyprland, "kitty's variables inherited by another terminal: not kitty's window");
        let no_socket = Env { vars: kitty_on_hyprland(shell_300()).vars.into_iter().filter(|(k, _)| k != "KITTY_LISTEN_ON").collect(), ..kitty_on_hyprland(shell_300()) };
        assert_eq!(detect(&no_socket), hyprland, "never through the terminal's own tty, which ttyak reads");
        assert_eq!(detect(&at(&[("HYPRLAND_INSTANCE_SIGNATURE", "abc")], &[300, 100], &[(0x5a, 999, "browser")])), Raise::None, "no parent of ttyak has a window");
        assert_eq!(describe(&kitty), "via kitty and Hyprland");
        assert_eq!(describe(&hyprland), "via Hyprland");
    }

    #[test]
    fn one_terminal_process_with_several_windows() {
        // kitty 100 has three OS windows; ttyak's tab is not the active one of its window yet.
        let windows = [(0x1, 100, "vim"), (0x2, 100, "htop"), (0x3, 100, "fish")];
        let vars = [("KITTY_WINDOW_ID", "5"), ("KITTY_LISTEN_ON", KITTY_TO), ("HYPRLAND_INSTANCE_SIGNATURE", "abc"), ("WAYLAND_DISPLAY", "wayland-1")];
        let kitty = Env { kitty: shell_300(), ..at(&vars, &[300, 100], &windows) };
        let later = Pick::Titled { pid: 100, title: "ttyak".into() };
        assert_eq!(detect(&kitty), Raise::Kitty { window: "5".into(), to: KITTY_TO.into(), then: Box::new(Raise::Hyprland { window: later }) }, "the window kitty retitles once it focused ttyak's tab");
        let shown = Env { windows: vec![Window { id: 0x2, pid: 100, title: "ttyak".into() }, Window { id: 0x1, pid: 100, title: "vim".into() }], ..kitty.clone() };
        assert_eq!(detect(&shown), Raise::Kitty { window: "5".into(), to: KITTY_TO.into(), then: Box::new(Raise::Hyprland { window: Pick::Id(0x2) }) }, "already titled so");
        // Without remote control: only the title ttyak set tells them apart, and only as it is now.
        let sway = |title: Option<&str>, windows: &[(u64, u32, &str)]| Env { title: title.map(String::from), ..at(&[("SWAYSOCK", "/s")], &[300, 100], windows) };
        assert_eq!(detect(&sway(Some("○ ttyak - w"), &[(7, 100, "fish"), (8, 100, "○ ttyak - w")])), Raise::Sway { window: Pick::Id(8) });
        assert_eq!(steps(&Raise::Sway { window: Pick::Id(8) }, &[]), vec![Step::Focus(Desktop::Sway, Pick::Id(8), Vec::new())]);
        assert_eq!(focus_command(Desktop::Sway, 8), ["swaymsg", "[con_id=8] focus"]);
        assert_eq!(detect(&sway(Some("○ ttyak - w"), &[(7, 100, "fish"), (8, 100, "vim")])), Raise::None, "ambiguous: the window manager is left out");
        assert_eq!(detect(&sway(None, &[(7, 100, "fish"), (8, 100, "vim")])), Raise::None, "no title known: left out");
        assert_eq!(detect(&sway(Some("x"), &[(7, 100, "x"), (8, 100, "x")])), Raise::None, "two of that title: left out");
        assert_eq!(titled(&[Window { id: 3, pid: 100, title: "t".into() }, Window { id: 4, pid: 101, title: "t".into() }], 100, "t"), Some(3));
    }

    #[test]
    fn tmux_switches_the_client_then_raises_the_outer_terminal() {
        let outer = Env { pid: 77, ..kitty_on_hyprland(shell_300()) };
        let inside = Env {
            tmux_client: Some(("/dev/pts/4".into(), Box::new(outer))),
            ..at(&[("TMUX", "/tmp/tmux-1000/default,77,0"), ("TMUX_PANE", "%3"), ("HYPRLAND_INSTANCE_SIGNATURE", "stale"), ("XDG_RUNTIME_DIR", "/run/user/1000")], &[400, 77], &[])
        };
        let tmux = detect(&inside);
        let kitty = Raise::Kitty { window: "5".into(), to: KITTY_TO.into(), then: Box::new(Raise::Hyprland { window: Pick::Id(0x6b) }) };
        let outer_vars: Vec<(String, String)> = [("HYPRLAND_INSTANCE_SIGNATURE", "abc"), ("DISPLAY", ":1"), ("WAYLAND_DISPLAY", "wayland-1")].iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
        assert_eq!(tmux, Raise::Tmux { pane: "%3".into(), client: "/dev/pts/4".into(), vars: outer_vars.clone(), then: Box::new(kitty) }, "the outer terminal's desktop variables, not the tmux server's");
        assert_eq!(
            steps(&tmux, &[]),
            vec![
                run(&["tmux", "select-window", "-t", "%3", ";", "select-pane", "-t", "%3", ";", "switch-client", "-c", "/dev/pts/4", "-t", "%3"]),
                Step::Run(argv(&["kitty", "@", "--to", KITTY_TO, "focus-window", "--match", "id:5"]), outer_vars.clone()),
                Step::Focus(Desktop::Hyprland, Pick::Id(0x6b), outer_vars),
            ]
        );
        assert_eq!(describe(&tmux), "via tmux, kitty and Hyprland");
        let detached = Env { tmux_client: None, ..inside };
        assert_eq!(detect(&detached), Raise::None, "no client: nothing on a screen to raise");
    }

    #[test]
    fn the_desktops_variables_and_x_authority() {
        let vars: HashMap<String, String> = [("DISPLAY", ":0"), ("HOME", "/home/a"), ("SWAYSOCK", "/s"), ("TERM", "xterm")].iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
        let carried = desktop_vars(&vars);
        assert_eq!(get(&carried, "XAUTHORITY"), Some("/home/a/.Xauthority"), "X's default file");
        assert_eq!((get(&carried, "DISPLAY"), get(&carried, "SWAYSOCK"), get(&carried, "TERM")), (Some(":0"), Some("/s"), None));
        // An .Xauthority with a cookie for another display, then a matching local one.
        let entry = |family: u16, address: &[u8], number: &[u8], name: &[u8], data: &[u8]| {
            let mut e = family.to_be_bytes().to_vec();
            for f in [address, number, name, data] {
                e.extend((f.len() as u16).to_be_bytes());
                e.extend(f);
            }
            e
        };
        let file = [entry(256, b"host", b"1", b"MIT-MAGIC-COOKIE-1", b"one"), entry(256, b"host", b"0", b"MIT-MAGIC-COOKIE-1", b"zero")].concat();
        let local = |f: u16| f == 256;
        assert_eq!(xauth_cookie(&file, local, b"host", 0), Some((b"MIT-MAGIC-COOKIE-1".to_vec(), b"zero".to_vec())));
        assert_eq!(xauth_cookie(&file, local, b"other", 0), None);
        assert_eq!(xauth_cookie(&file, |f| f == 0, b"host", 0), None, "another family");
        assert_eq!(xauth_cookie(&file[..5], local, b"host", 0), None, "a cut file");
    }

    #[test]
    fn sway_and_niri_by_the_terminals_window() {
        let sway = at(&[("SWAYSOCK", "/run/user/1000/sway-ipc.sock"), ("WAYLAND_DISPLAY", "wayland-1")], &[300, 200], &[(7, 200, "foot")]);
        assert_eq!(detect(&sway), Raise::Sway { window: Pick::Id(7) });
        let niri = at(&[("NIRI_SOCKET", "/run/user/1000/niri.sock"), ("WAYLAND_DISPLAY", "wayland-1")], &[300, 200], &[(9, 111, "a"), (12, 200, "b")]);
        assert_eq!(detect(&niri), Raise::Niri { window: Pick::Id(12) });
        assert_eq!(focus_command(Desktop::Niri, 12), ["niri", "msg", "action", "focus-window", "--id", "12"]);
        assert_eq!(describe(&Raise::Sway { window: Pick::Id(7) }), "via sway");
    }

    #[test]
    fn wezterm_activates_the_pane() {
        let env = Env { title: Some("ttyak".into()), ..at(&[("WEZTERM_PANE", "2"), ("WEZTERM_UNIX_SOCKET", "/run/user/1000/wezterm/gui-sock-9"), ("SWAYSOCK", "/s")], &[300, 9], &[(4, 9, "x"), (5, 9, "y")]) };
        let r = detect(&env);
        assert_eq!(r, Raise::Wezterm { pane: "2".into(), socket: Some("/run/user/1000/wezterm/gui-sock-9".into()), then: Box::new(Raise::Sway { window: Pick::Titled { pid: 9, title: "ttyak".into() } }) });
        assert_eq!(steps(&r, &[])[0], Step::Run(argv(&["wezterm", "cli", "activate-pane", "--pane-id", "2"]), vec![("WEZTERM_UNIX_SOCKET".into(), "/run/user/1000/wezterm/gui-sock-9".into())]));
    }

    #[test]
    fn x11_by_windowid_or_the_window_of_a_parent() {
        assert_eq!(detect(&at(&[("DISPLAY", ":0"), ("WINDOWID", "41943047")], &[300, 100], &[])), Raise::X11 { window: Pick::Id(41943047) });
        assert_eq!(detect(&at(&[("DISPLAY", ":0")], &[300, 100], &[(0x1c00007, 100, "xterm")])), Raise::X11 { window: Pick::Id(0x1c00007) });
        assert_eq!(steps(&Raise::X11 { window: Pick::Id(5) }, &[]), vec![Step::Focus(Desktop::X11, Pick::Id(5), Vec::new())]);
        assert_eq!(detect(&at(&[("DISPLAY", ":0"), ("WAYLAND_DISPLAY", "wayland-0"), ("WINDOWID", "41943047")], &[300], &[])), Raise::None, "XWayland's DISPLAY is no X11 desktop");
        let vars = |v: &[(&str, &str)]| v.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect::<HashMap<_, _>>();
        assert_eq!(listed(&vars(&[("DISPLAY", ":0")]), "linux"), Some(Desktop::X11));
        assert_eq!(listed(&vars(&[("DISPLAY", "localhost:10.0"), ("SSH_CONNECTION", "10.0.0.2 51000 10.0.0.1 22")]), "linux"), None, "ssh -X: the parents are remote processes");
        assert_eq!(listed(&vars(&[("DISPLAY", ":0"), ("WINDOWID", "7")]), "linux"), None, "WINDOWID is exact");
        assert_eq!(listed(&vars(&[("SWAYSOCK", "/s")]), "linux"), Some(Desktop::Sway));
    }

    #[test]
    fn kde_through_a_kwin_script() {
        let r = detect(&Env { title: Some("ttyak".into()), ..at(&[("KDE_SESSION_VERSION", "6"), ("WAYLAND_DISPLAY", "wayland-0"), ("XDG_CURRENT_DESKTOP", "KDE")], &[300, 100], &[]) });
        assert_eq!(r, Raise::Kwin { pids: vec![300, 100], title: Some("ttyak".into()), plasma6: true });
        assert_eq!(steps(&r, &[]), vec![Step::Kwin { pids: vec![300, 100], title: Some("ttyak".into()), plasma6: true, vars: Vec::new() }]);
        assert_eq!(detect(&at(&[("KDE_SESSION_VERSION", "5"), ("DISPLAY", ":0")], &[300], &[])), Raise::Kwin { pids: vec![300], title: None, plasma6: false });
        assert_eq!(detect(&at(&[("KDE_SESSION_VERSION", "6")], &[], &[])), Raise::None, "no parents known: nothing for the script to find");
        let script = kwin_script(&[300, 100], Some("a \"quoted\" title"));
        assert!(script.contains("[300, 100]") && script.contains(r#"var title = "a \"quoted\" title";"#) && script.contains("workspace.activeWindow") && script.contains("workspace.activeClient"), "{script}");
        assert!(kwin_script(&[1], None).contains("var title = null;"));
        assert_eq!(kwin_script_id("method return time=1.2 sender=:1.5 -> destination=:1.9 serial=7 reply_serial=2\n   int32 3\n"), Some(3));
        assert_eq!(kwin_script_id("method return time=1.2\n   int32 -1\n"), None);
        assert_eq!(describe(&r), "via KWin");
    }

    #[test]
    fn gnome_on_wayland_cannot() {
        let gnome = at(&[("XDG_CURRENT_DESKTOP", "GNOME"), ("WAYLAND_DISPLAY", "wayland-0"), ("DISPLAY", ":0")], &[300, 100], &[]);
        assert_eq!(detect(&gnome), Raise::None);
        assert!(steps(&Raise::None, &[]).is_empty());
        assert_eq!(describe(&Raise::None), CANNOT);
        let mac = Env { os: "macos", ..at(&[("DISPLAY", ":0"), ("WINDOWID", "7")], &[300], &[]) };
        assert_eq!(detect(&mac), Raise::None, "macOS and Windows have routes of their own");
    }

    #[test]
    fn what_the_tools_say() {
        // Shapes as the tools print them (Hyprland 0.56's and kitty 0.48's captured here, trimmed; sway's and niri's
        // from their IPC docs).
        let hyprland = r#"[{"address": "0x56c1d9bfba40", "mapped": true, "pid": 625833, "class": "kitty", "title": "a"}, {"address": "0x56c1d9e331b0", "pid": 1545088, "title": "b"}]"#;
        assert_eq!(hyprland_windows(hyprland), vec![Window { id: 0x56c1d9bfba40, pid: 625833, title: "a".into() }, Window { id: 0x56c1d9e331b0, pid: 1545088, title: "b".into() }]);
        let sway = r#"{"id": 1, "type": "root", "nodes": [{"id": 3, "type": "output", "nodes": [{"id": 4, "type": "workspace", "nodes": [{"id": 6, "type": "con", "pid": 812, "name": "foot", "nodes": []}], "floating_nodes": [{"id": 8, "type": "floating_con", "pid": 813, "name": null, "nodes": []}]}]}]}"#;
        assert_eq!(sway_windows(sway), vec![Window { id: 6, pid: 812, title: "foot".into() }, Window { id: 8, pid: 813, title: String::new() }]);
        let niri = r#"[{"id": 12, "title": "x", "app_id": "kitty", "pid": 200, "workspace_id": 1, "is_focused": false}, {"id": 13, "pid": null}]"#;
        assert_eq!(niri_windows(niri), vec![Window { id: 12, pid: 200, title: "x".into() }]);
        assert!(hyprland_windows("not json").is_empty());
        let kitty = r#"[{"id": 1, "is_active": true, "tabs": [{"id": 2, "title": "tab", "windows": [{"id": 4, "pid": 10, "title": "other"}, {"id": 5, "pid": 2201628, "title": "○ ttyak - w", "is_self": false}]}]}]"#;
        assert_eq!(kitty_window(kitty, "5"), Some(KittyWindow { pid: 2201628, title: "○ ttyak - w".into() }));
        assert_eq!(kitty_window(kitty, "9"), None);
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
    /// focus (a headless compositor in a container). `TTYAK_RAISE_TITLE`: the title ttyak would have set.
    #[test]
    #[ignore]
    fn live_raise() {
        if std::env::var("TTYAK_RAISE_LIVE").as_deref() != Ok("1") {
            return;
        }
        // Every command and how it ended.
        let _ = tracing_subscriber::fmt().with_max_level(tracing::Level::DEBUG).with_ansi(false).without_time().with_writer(std::io::stdout).try_init();
        let r = detect(&Env { title: std::env::var("TTYAK_RAISE_TITLE").ok(), ..env() });
        println!("{r:?} -> {}", describe(&r));
        raise(&r);
    }
}
