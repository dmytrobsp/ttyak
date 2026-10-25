//! The status file: a running ttyak writes what the badge says (read, unread, for you), per workspace, whether it
//! is paused, in a huddle, or has an update waiting, for `ttyak ctl status` to read (status bars, scripts). Never
//! message text or authors.

use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::tui::attention::{self, State};
use super::tui::others::Badge;
use super::tui::state::Workspace;
use super::tui::title;

/// What `ctl status` shows. `version` is the schema's.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Status {
    pub version: u32,
    pub running: bool,
    pub pid: u32,
    pub profile: String,
    /// `read`, `unread` or `for_you` (empty when not running).
    pub state: String,
    /// `○`, `●` or `◆` (empty when not running).
    pub mark: String,
    /// Everything that is for you, in every workspace of the profile.
    pub for_you: u64,
    pub workspaces: Vec<WorkspaceStatus>,
    /// RFC 3339: when the pause or the schedule's quiet time ends.
    pub paused_until: Option<String>,
    pub huddle: Option<HuddleStatus>,
    /// The version of an update that is waiting.
    pub update: Option<String>,
    /// RFC 3339, when this content was written.
    pub updated_at: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct WorkspaceStatus {
    pub name: String,
    pub current: bool,
    pub unread: bool,
    pub for_you: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct HuddleStatus {
    /// The conversation the huddle is in.
    #[serde(rename = "where")]
    pub place: String,
}

/// The words of the line, for `tr`.
pub const WORDS: &[&str] = &["ttyak is not running", "all read", "unread", "{n} for you", "paused until {t}", "in a huddle ({where})", "update {v}"];

impl Status {
    /// What is read when no ttyak runs the profile.
    pub fn not_running(profile: &str) -> Status {
        Status { version: 1, running: false, pid: 0, profile: profile.into(), state: String::new(), mark: String::new(), for_you: 0, workspaces: Vec::new(), paused_until: None, huddle: None, update: None, updated_at: String::new() }
    }
}

/// Where profile `profile`'s file is: `$XDG_RUNTIME_DIR/ttyak/<profile>.json`, else `<cache>/<profile>/status.json`.
pub fn path(profile: &str) -> PathBuf {
    let cache = slack_core::session::project_dirs().map_or_else(|| std::env::temp_dir().join("ttyak"), |d| d.cache_dir().to_path_buf());
    path_in(std::env::var_os("XDG_RUNTIME_DIR").as_deref(), &cache, profile)
}

/// [`path`] with the runtime folder and the cache folder given.
fn path_in(runtime: Option<&std::ffi::OsStr>, cache: &Path, profile: &str) -> PathBuf {
    match runtime.filter(|r| !r.is_empty()) {
        Some(r) => Path::new(r).join("ttyak").join(format!("{profile}.json")),
        None => cache.join(profile).join("status.json"),
    }
}

/// Writes `s` to profile `profile`'s file.
pub fn write(profile: &str, s: &Status) -> io::Result<()> {
    write_to(&path(profile), s)
}

/// Writes `s` as a temporary file (0600) renamed over `file`, so a reader never sees half of it.
fn write_to(file: &Path, s: &Status) -> io::Result<()> {
    use std::io::Write;
    let dir = file.parent().unwrap_or(Path::new("."));
    if !dir.is_dir() {
        std::fs::create_dir_all(dir)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700))?;
        }
    }
    let temp = file.with_extension(format!("tmp{}", std::process::id()));
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut f = options.open(&temp)?;
    let done = f.write_all(serde_json::to_string(s)?.as_bytes()).and_then(|()| f.flush());
    drop(f);
    match done.and_then(|()| std::fs::rename(&temp, file)) {
        Ok(()) => Ok(()),
        Err(e) => {
            let _ = std::fs::remove_file(&temp);
            Err(e)
        }
    }
}

/// Takes profile `profile`'s file away (a clean quit).
pub fn remove(profile: &str) {
    let _ = std::fs::remove_file(path(profile));
}

/// What profile `profile`'s ttyak last wrote; not running when there is no file, it cannot be read, or its ttyak is gone.
pub fn read(profile: &str) -> Status {
    read_from(&path(profile), profile)
}

fn read_from(file: &Path, profile: &str) -> Status {
    let s: Option<Status> = std::fs::read(file).ok().and_then(|b| serde_json::from_slice(&b).ok());
    match s {
        Some(s) if s.running && alive(s.pid, profile) => s,
        _ => Status::not_running(profile),
    }
}

/// Whether process `pid` exists (Windows: whether a ttyak holds the profile's lock).
fn alive(pid: u32, profile: &str) -> bool {
    #[cfg(unix)]
    {
        let _ = profile;
        // Signal 0 only checks; no permission to signal it still means it exists.
        i32::try_from(pid).is_ok_and(|p| p > 0 && (unsafe { libc::kill(p, 0) } == 0 || std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)))
    }
    #[cfg(not(unix))]
    {
        let _ = pid;
        slack_core::profiles::Dirs::system().is_ok_and(|d| slack_core::profiles::is_running(&d, profile))
    }
}

/// The profile this ttyak runs, and the content last written (without its time), so a file is written once per change.
static RUNNING: std::sync::Mutex<Option<(String, String)>> = std::sync::Mutex::new(None);

/// This ttyak runs profile `name`: its file goes when the returned guard drops (a clean quit, an error), and a
/// leftover one of an earlier run is read as not running meanwhile (its pid is gone).
pub fn start(name: &str) -> Guard {
    *RUNNING.lock().unwrap_or_else(|e| e.into_inner()) = Some((name.to_string(), String::new()));
    Guard
}

pub struct Guard;

impl Drop for Guard {
    fn drop(&mut self) {
        if let Some((profile, _)) = RUNNING.lock().unwrap_or_else(|e| e.into_inner()).take() {
            remove(&profile);
        }
    }
}

/// The status of the shown workspace `ws` (number `current` of `labels`) and the other workspaces' `others`.
pub fn snapshot(ws: &Workspace, labels: &[String], current: usize, others: &HashMap<usize, Badge>, huddle: Option<String>, update: Option<String>) -> Option<Status> {
    let profile = RUNNING.lock().unwrap_or_else(|e| e.into_inner()).as_ref()?.0.clone();
    let state = attention::state(ws, others);
    // Slack's own counts: mentions in the conversations and threads; an unread DM is at least one.
    let mut here = ws.total_unread().1 + ws.thread_unread.mentions;
    if here == 0 && ws.dm_unread() {
        here = 1;
    }
    let shown = WorkspaceStatus { name: String::new(), current: true, unread: attention::state(ws, &HashMap::new()) != State::Read, for_you: here };
    let workspaces: Vec<WorkspaceStatus> = labels
        .iter()
        .enumerate()
        .map(|(i, label)| {
            let name = title::clean(label);
            match others.get(&i).filter(|_| i != current) {
                _ if i == current => WorkspaceStatus { name, ..shown.clone() },
                Some(b) => WorkspaceStatus { name, current: false, unread: b.unread || b.attention > 0, for_you: b.attention },
                None => WorkspaceStatus { name, current: false, unread: false, for_you: 0 },
            }
        })
        .collect();
    let now = super::tui::clock::now();
    Some(Status {
        version: 1,
        running: true,
        pid: std::process::id(),
        profile,
        state: state.name().into(),
        mark: state.mark().into(),
        for_you: workspaces.iter().map(|w| w.for_you).sum(),
        workspaces,
        paused_until: ws.dnd_until(now.timestamp() as f64).and_then(|t| super::tui::clock::at(t as i64)).map(|t| t.to_rfc3339()),
        huddle: huddle.map(|place| HuddleStatus { place: title::clean(&place) }),
        update,
        updated_at: now.to_rfc3339(),
    })
}

/// Writes `s` unless it says what was last written (its time aside).
pub fn publish(mut s: Status) {
    let at = std::mem::take(&mut s.updated_at);
    let Ok(content) = serde_json::to_string(&s) else { return };
    let mut last = RUNNING.lock().unwrap_or_else(|e| e.into_inner());
    let Some((profile, written)) = last.as_mut() else { return };
    if *written == content {
        return;
    }
    s.updated_at = at;
    // A failed write is tried again at the next change.
    if write(profile, &s).is_ok() {
        *written = content;
    }
}

/// `ttyak ctl status`: prints the status once, or on every change with `follow` (checked every `every` seconds).
pub async fn print(profile: &str, json: bool, follow: bool, every: u64) -> anyhow::Result<()> {
    use std::io::Write;
    if let Err(e) = super::tui::i18n::set(&super::tui::settings::load().0.region.language) {
        tracing::debug!("language: {e}");
    }
    let mut last = None;
    loop {
        let s = read(profile);
        let out = if json { serde_json::to_string(&s)? } else { line(&s) };
        if last.as_ref() != Some(&out) {
            let mut stdout = std::io::stdout().lock();
            // A reader that has gone (a closed pipe) ends it.
            if writeln!(stdout, "{out}").and_then(|()| stdout.flush()).is_err() || !follow {
                return Ok(());
            }
            last = Some(out);
        } else if !follow {
            return Ok(());
        }
        tokio::time::sleep(std::time::Duration::from_secs(every.max(1))).await;
    }
}

/// The one-line form, as the bars show it: `◆ acme: 2 for you · paused until 14:00`.
pub fn line(s: &Status) -> String {
    line_at(s, chrono::Utc::now().fixed_offset())
}

/// [`line`] as of `now` (a pause ending today shows the time alone, another day with the weekday).
fn line_at(s: &Status, now: chrono::DateTime<chrono::FixedOffset>) -> String {
    use super::tui::i18n::tr;
    if !s.running {
        return tr("ttyak is not running").into_owned();
    }
    let named = |keep: &dyn Fn(&WorkspaceStatus) -> bool| s.workspaces.iter().filter(|w| keep(w)).map(|w| super::tui::title::clean(&w.name)).collect::<Vec<_>>().join(", ");
    let (names, what) = match s.state.as_str() {
        "for_you" => (named(&|w| w.for_you > 0), tr("{n} for you").replace("{n}", &s.for_you.to_string())),
        "unread" => (named(&|w| w.unread), tr("unread").into_owned()),
        _ => (named(&|w| w.current), tr("all read").into_owned()),
    };
    let mut out = format!("{} {names}: {what}", s.mark);
    if let Some(until) = s.paused_until.as_deref().and_then(|t| chrono::DateTime::parse_from_rfc3339(t).ok()) {
        let format = if until.date_naive() == now.with_timezone(until.offset()).date_naive() { "%H:%M" } else { "%a %H:%M" };
        out += &format!(" · {}", tr("paused until {t}").replace("{t}", &until.format(format).to_string()));
    }
    if let Some(h) = &s.huddle {
        out += &format!(" · {}", tr("in a huddle ({where})").replace("{where}", &super::tui::title::clean(&h.place)));
    }
    if let Some(v) = &s.update {
        out += &format!(" · {}", tr("update {v}").replace("{v}", v));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ws(name: &str, current: bool, unread: bool, for_you: u64) -> WorkspaceStatus {
        WorkspaceStatus { name: name.into(), current, unread, for_you }
    }

    fn status(state: &str, workspaces: Vec<WorkspaceStatus>) -> Status {
        let mark = match state {
            "for_you" => "◆",
            "unread" => "●",
            _ => "○",
        };
        let for_you = workspaces.iter().map(|w| w.for_you).sum();
        Status { version: 1, running: true, pid: std::process::id(), profile: "default".into(), state: state.into(), mark: mark.into(), for_you, workspaces, paused_until: None, huddle: None, update: None, updated_at: "2026-10-25T12:00:00+02:00".into() }
    }

    #[test]
    fn the_line_for_each_state() {
        assert_eq!(line(&status("read", vec![ws("acme", true, false, 0)])), "○ acme: all read");
        assert_eq!(line(&status("unread", vec![ws("acme", true, true, 0)])), "● acme: unread");
        assert_eq!(line(&status("for_you", vec![ws("acme", true, true, 2)])), "◆ acme: 2 for you");
        let mut two = status("for_you", vec![ws("acme", true, false, 0), ws("beta", false, true, 1)]);
        assert_eq!(line(&two), "◆ beta: 1 for you");
        two.huddle = Some(HuddleStatus { place: "ann".into() });
        two.update = Some("0.13.0".into());
        assert_eq!(line(&two), "◆ beta: 1 for you · in a huddle (ann) · update 0.13.0");
        assert_eq!(line(&Status::not_running("default")), "ttyak is not running");
    }

    #[test]
    fn the_line_names_the_pause() {
        let mut s = status("for_you", vec![ws("acme", true, true, 2)]);
        s.paused_until = Some("2099-01-01T14:00:00+02:00".into());
        let l = line(&s);
        assert!(l.starts_with("◆ acme: 2 for you · paused until ") && l.ends_with("14:00"), "{l}");
        // Today: the time alone.
        let now = chrono::Utc::now().with_timezone(&chrono::FixedOffset::east_opt(0).unwrap());
        s.paused_until = Some((now + chrono::Duration::minutes(5)).to_rfc3339());
        assert_eq!(line_at(&s, now), format!("◆ acme: 2 for you · paused until {}", (now + chrono::Duration::minutes(5)).format("%H:%M")));
    }

    #[test]
    fn a_name_is_cleaned() {
        let l = line(&status("read", vec![ws("ac\u{1b}[31mme\u{202e}\n", true, false, 0)]));
        assert_eq!(l, "○ ac[31mme: all read");
    }

    #[test]
    fn json_has_the_documented_names() {
        let mut s = status("unread", vec![ws("acme", true, true, 0)]);
        s.huddle = Some(HuddleStatus { place: "general".into() });
        let v = serde_json::to_value(&s).unwrap();
        assert_eq!(v["huddle"]["where"], "general");
        for k in ["version", "running", "pid", "profile", "state", "mark", "for_you", "workspaces", "paused_until", "huddle", "update", "updated_at"] {
            assert!(v.get(k).is_some(), "{k}");
        }
        assert_eq!(v["paused_until"], serde_json::Value::Null);
        assert_eq!(serde_json::from_value::<Status>(v).unwrap(), s);
    }

    #[test]
    fn path_follows_the_runtime_dir() {
        let cache = Path::new("/cache");
        assert_eq!(path_in(Some("/run/user/1000".as_ref()), cache, "b"), Path::new("/run/user/1000/ttyak/b.json"));
        assert_eq!(path_in(None, cache, "b"), Path::new("/cache/b/status.json"));
        assert_eq!(path_in(Some("".as_ref()), cache, "b"), Path::new("/cache/b/status.json"));
    }

    fn scratch(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("ttyak-status-{name}-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn a_file_of_a_gone_pid_reads_as_not_running() {
        let dir = scratch("gone");
        let file = dir.join("p.json");
        // A pid that cannot be alive.
        let mut s = status("for_you", vec![ws("acme", true, true, 1)]);
        s.pid = u32::MAX - 1;
        write_to(&file, &s).unwrap();
        assert!(!read_from(&file, "p").running);
        // Ours is alive.
        s.pid = std::process::id();
        write_to(&file, &s).unwrap();
        assert_eq!(read_from(&file, "p"), s);
        // Missing and unreadable files.
        assert!(!read_from(&dir.join("none.json"), "p").running);
        std::fs::write(&file, "{ not json").unwrap();
        assert!(!read_from(&file, "p").running);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn the_file_is_the_users_alone() {
        use std::os::unix::fs::PermissionsExt;
        let dir = scratch("mode");
        let file = dir.join("ttyak").join("p.json");
        write_to(&file, &status("read", vec![])).unwrap();
        assert_eq!(std::fs::metadata(&file).unwrap().permissions().mode() & 0o777, 0o600);
        assert_eq!(std::fs::metadata(file.parent().unwrap()).unwrap().permissions().mode() & 0o777, 0o700);
        // Written again over the old one: still 0600, no temp file left.
        write_to(&file, &status("unread", vec![])).unwrap();
        assert_eq!(std::fs::metadata(&file).unwrap().permissions().mode() & 0o777, 0o600);
        assert_eq!(std::fs::read_dir(file.parent().unwrap()).unwrap().count(), 1);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
