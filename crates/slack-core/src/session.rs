//! Browser-session credentials (`xoxc-…` token + `d` cookie) and how to obtain them.
//!
//! The Slack web/desktop client authenticates API calls with a per-workspace `xoxc`
//! token plus the `d` session cookie (see `docs/protocol/auth.md`).
//! Tokens are never logged; session files are written owner-only.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, bail};
use serde::{Deserialize, Serialize};

/// User-Agent of the official Linux desktop app; the web endpoints expect a browser.
pub const USER_AGENT: &str = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Slack/4.52.178 Chrome/152.0.7977.130 Electron/44.4.5 Safari/537.36 Sonic Slack_SSB/4.52.178";

/// Credentials for one workspace.
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct WorkspaceSession {
    pub team_id: String,
    pub team_name: String,
    /// e.g. `https://acme.slack.com/`
    pub url: String,
    pub user_id: String,
    pub user_name: String,
    pub token: String,
    /// Raw value of the `d` cookie as sent by browsers (already URL-encoded).
    pub cookie_d: String,
    /// Where the session came from, to offer it first when signing in again (none in files
    /// written before it was remembered).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<SessionSource>,
}

/// How a session was obtained: names and paths only, nothing secret.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SessionSource {
    /// Taken from a browser profile the user is signed in with: `browser` is `firefox` or
    /// `chrome`, `profile` the profile's folder, `label` how it is shown ("Chrome · Work").
    Browser { browser: String, profile: PathBuf, label: String },
    /// Signed in in ttyak's former temporary browser window (no longer offered; old files still load).
    Window,
    /// The `d` cookie given by hand (flag, environment or prompt).
    Cookie,
    /// An emailed sign-in code, confirmed by ttyak (`email` is where Slack sent it).
    Code { email: String },
}

impl std::fmt::Debug for WorkspaceSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WorkspaceSession")
            .field("team_id", &self.team_id)
            .field("team_name", &self.team_name)
            .field("url", &self.url)
            .field("user_id", &self.user_id)
            .field("user_name", &self.user_name)
            .field("token", &"<redacted>")
            .field("cookie_d", &"<redacted>")
            .field("source", &self.source)
            .finish()
    }
}

impl WorkspaceSession {
    pub fn api_base(&self) -> String {
        format!("{}/api/", self.url.trim_end_matches('/'))
    }

    pub fn cookie_header(&self) -> String {
        format!("d={}", self.cookie_d)
    }
}

/// All workspaces of one profile.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Profile {
    pub workspaces: Vec<WorkspaceSession>,
    /// Team id opened at start-up.
    #[serde(default)]
    pub default_team: Option<String>,
}

/// ttyak's folders, plain "ttyak" on every platform: `~/.config/ttyak`, `~/.cache/ttyak` and
/// `~/.local/share/ttyak` on Linux, `~/Library/{Application Support,Caches}/ttyak` on macOS,
/// `%APPDATA%\ttyak` and `%LOCALAPPDATA%\ttyak` on Windows.
pub fn project_dirs() -> Option<directories::ProjectDirs> {
    directories::ProjectDirs::from_path(PathBuf::from("ttyak"))
}

pub fn config_dir(profile: &str) -> Result<PathBuf> {
    let dirs = project_dirs().ok_or_else(|| anyhow!("no home directory"))?;
    Ok(dirs.config_dir().join(profile))
}

pub fn cache_dir(profile: &str) -> Result<PathBuf> {
    let dirs = project_dirs().ok_or_else(|| anyhow!("no home directory"))?;
    Ok(dirs.cache_dir().join(profile))
}

impl Profile {
    fn path(profile: &str) -> Result<PathBuf> {
        Ok(config_dir(profile)?.join("session.json"))
    }

    pub fn load(profile: &str) -> Result<Self> {
        Self::load_at(profile, &Self::path(profile)?)
    }

    /// [`Profile::load`] from the file `path`: a file that cannot be read names itself.
    fn load_at(profile: &str, path: &Path) -> Result<Self> {
        let text = std::fs::read_to_string(path).with_context(|| format!("no session for profile '{profile}' — run `ttyak auth login`"))?;
        serde_json::from_str(&text).with_context(|| format!("cannot read {}", path.display()))
    }

    /// Read a session file, or a new empty profile when there is none; a file that exists but
    /// cannot be read is an error, so that saving does not replace its sessions.
    pub fn read_or_new(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Profile::default());
        }
        Self::read(path).with_context(|| format!("cannot read {}", path.display()))
    }

    /// Read a session file (another profile's, for `ttyak profile list`).
    pub fn read(path: &Path) -> Result<Self> {
        Ok(serde_json::from_str(&std::fs::read_to_string(path)?)?)
    }

    pub fn save(&self, profile: &str) -> Result<PathBuf> {
        let path = Self::path(profile)?;
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        write_private(&path, serde_json::to_string_pretty(self)?.as_bytes())?;
        Ok(path)
    }

    /// Insert or replace the session of its account (team + user: one profile may hold two
    /// accounts of one workspace).
    /// The account keeps its place, so the one started and the Alt+1…9 order stay.
    pub fn upsert(&mut self, ws: WorkspaceSession) {
        if self.default_team.is_none() {
            self.default_team = Some(ws.team_id.clone());
        }
        match self.workspaces.iter_mut().find(|w| w.team_id == ws.team_id && w.user_id == ws.user_id) {
            Some(old) => *old = ws,
            None => self.workspaces.push(ws),
        }
    }

    /// Every account of the workspace `team` (its id or name).
    pub fn matching(&self, team: &str) -> Vec<&WorkspaceSession> {
        self.workspaces.iter().filter(|w| w.team_id == team || w.team_name == team).collect()
    }

    /// Forget the account `user_id` of `team_id`; false when there is none. When its team was the
    /// default and has no account left, the first remaining workspace becomes the default.
    pub fn remove(&mut self, team_id: &str, user_id: &str) -> bool {
        let before = self.workspaces.len();
        self.workspaces.retain(|w| w.team_id != team_id || w.user_id != user_id);
        if self.default_team.as_deref().is_some_and(|t| !self.workspaces.iter().any(|w| w.team_id == t)) {
            self.default_team = self.workspaces.first().map(|w| w.team_id.clone());
        }
        self.workspaces.len() != before
    }

    pub fn workspace(&self, team: Option<&str>) -> Option<&WorkspaceSession> {
        let team = team.or(self.default_team.as_deref());
        match team {
            Some(t) => self.workspaces.iter().find(|w| w.team_id == t || w.team_name == t),
            None => self.workspaces.first(),
        }
    }
}

/// Write `bytes` to `path` readable by the owner only, atomically: through a temporary file of this write's own beside
/// it (`.<name>.<pid>.<n>.tmp`), so writers at once (two profiles' ttyaks) never mix their files; removed again when
/// the write fails.
pub fn write_private(path: &Path, bytes: &[u8]) -> Result<()> {
    write_private_with(path, bytes, owner_only)
}

/// [`write_private`] making the empty temporary file owner-only with `protect` (a failure is logged and the write goes
/// on: losing a sign-in would be worse).
fn write_private_with(path: &Path, bytes: &[u8], protect: fn(&Path, bool) -> std::io::Result<()>) -> Result<()> {
    static WRITES: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let name = path.file_name().map_or_else(|| "file".into(), |n| n.to_string_lossy().into_owned());
    let n = WRITES.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let tmp = path.with_file_name(format!(".{name}.{}.{n}.tmp", std::process::id()));
    let written = (|| -> std::io::Result<()> {
        {
            #[cfg(unix)]
            let mut f = {
                use std::os::unix::fs::OpenOptionsExt;
                std::fs::OpenOptions::new().write(true).create(true).truncate(true).mode(0o600).open(&tmp)?
            };
            #[cfg(not(unix))]
            let mut f = std::fs::File::create(&tmp)?;
            // Before the bytes are in it; the rename keeps the ACL.
            logged(&tmp, protect(&tmp, false));
            std::io::Write::write_all(&mut f, bytes)?;
        }
        std::fs::rename(&tmp, path)
    })();
    if written.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    Ok(written?)
}

/// [`owner_only`], a failure logged at debug and the caller going on (a file system without ACLs — FAT, some network
/// shares — still gets the file: losing a sign-in would be worse).
pub fn owner_only_or_log(path: &Path, dir: bool) {
    logged(path, owner_only(path, dir));
}

/// Logs the failure of making `path` owner-only, if any.
fn logged(path: &Path, made: std::io::Result<()>) {
    if let Err(e) = made {
        tracing::debug!("owner-only {}: {e}", path.display());
    }
}

/// Make `path` (a folder when `dir`) the user's alone. Windows: a protected DACL granting full access to the user,
/// SYSTEM and Administrators only, passed on to what a folder holds. Elsewhere nothing: files are made with their mode.
#[cfg(windows)]
pub fn owner_only(path: &Path, dir: bool) -> std::io::Result<()> {
    use windows_sys::Win32::Foundation::LocalFree;
    use windows_sys::Win32::Security::Authorization::{ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1, SE_FILE_OBJECT, SetNamedSecurityInfoW};
    use windows_sys::Win32::Security::{ACL, DACL_SECURITY_INFORMATION, GetSecurityDescriptorDacl, PROTECTED_DACL_SECURITY_INFORMATION};
    let sddl = windows::wide(std::ffi::OsStr::new(&owner_only_sddl(&user_sid()?, dir)));
    let name = windows::wide(path.as_os_str());
    let mut sd = std::ptr::null_mut();
    // SAFETY: the strings end with a NUL; `sd` is freed with LocalFree once its DACL is set.
    unsafe {
        if ConvertStringSecurityDescriptorToSecurityDescriptorW(sddl.as_ptr(), SDDL_REVISION_1, &mut sd, std::ptr::null_mut()) == 0 {
            return Err(std::io::Error::last_os_error());
        }
        let (mut present, mut defaulted, mut dacl) = (0, 0, std::ptr::null_mut::<ACL>());
        let set = if GetSecurityDescriptorDacl(sd, &mut present, &mut dacl, &mut defaulted) == 0 {
            Err(std::io::Error::last_os_error())
        } else if present == 0 || dacl.is_null() {
            // Fail closed: a null DACL would grant everyone everything.
            Err(std::io::Error::other("no DACL in the owner-only descriptor"))
        } else {
            match SetNamedSecurityInfoW(name.as_ptr(), SE_FILE_OBJECT, DACL_SECURITY_INFORMATION | PROTECTED_DACL_SECURITY_INFORMATION, std::ptr::null_mut(), std::ptr::null_mut(), dacl, std::ptr::null()) {
                0 => Ok(()),
                e => Err(std::io::Error::from_raw_os_error(e as i32)),
            }
        };
        LocalFree(sd);
        set
    }
}

/// Make `path` (a folder when `dir`) the user's alone: nothing to do here, files are made with their mode.
#[cfg(not(windows))]
pub fn owner_only(_path: &Path, _dir: bool) -> std::io::Result<()> {
    Ok(())
}

/// The SDDL of [`owner_only`]'s DACL: protected, full access for `user_sid`, SYSTEM and Administrators (as
/// Win32-OpenSSH's private keys), inherited by files and folders inside when `dir`.
#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) fn owner_only_sddl(user_sid: &str, dir: bool) -> String {
    let inherit = if dir { "OICI" } else { "" };
    format!("D:P(A;{inherit};FA;;;{user_sid})(A;{inherit};FA;;;SY)(A;{inherit};FA;;;BA)")
}

/// The SID of the user this process runs as (its token's user, not its default owner: for an elevated admin that may
/// be Administrators), as a string; asked once (the status file is written often).
#[cfg(windows)]
pub fn user_sid() -> std::io::Result<String> {
    static SID: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    if let Some(sid) = SID.get() {
        return Ok(sid.clone());
    }
    let sid = token_user_sid()?;
    Ok(SID.get_or_init(|| sid).clone())
}

/// [`user_sid`] asked of the process token.
#[cfg(windows)]
fn token_user_sid() -> std::io::Result<String> {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::Security::{GetTokenInformation, TOKEN_QUERY, TOKEN_USER, TokenUser};
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};
    let mut token = std::ptr::null_mut();
    // SAFETY: the token is closed below; the buffer is u64s, aligned for TOKEN_USER, of the size asked for.
    unsafe {
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) == 0 {
            return Err(std::io::Error::last_os_error());
        }
        let mut len = 0;
        GetTokenInformation(token, TokenUser, std::ptr::null_mut(), 0, &mut len);
        let mut buf = vec![0u64; (len as usize).div_ceil(8).max(1)];
        let got = GetTokenInformation(token, TokenUser, buf.as_mut_ptr().cast(), len, &mut len);
        let e = std::io::Error::last_os_error();
        CloseHandle(token);
        if got == 0 {
            return Err(e);
        }
        windows::sid_string((*buf.as_ptr().cast::<TOKEN_USER>()).User.Sid)
    }
}

/// The type of an ACE that allows (`ACCESS_ALLOWED_ACE_TYPE`), for the tests of [`owner_only`].
#[cfg(windows)]
#[doc(hidden)]
pub const ACCESS_ALLOWED: u8 = 0;

/// What `path`'s DACL says, for the tests of [`owner_only`]: whether it is protected (inherits nothing), and each
/// ACE's type, flags and SID.
#[cfg(windows)]
#[doc(hidden)]
pub fn dacl_of(path: &Path) -> std::io::Result<(bool, Vec<(u8, u8, String)>)> {
    use windows_sys::Win32::Foundation::LocalFree;
    use windows_sys::Win32::Security::Authorization::{GetNamedSecurityInfoW, SE_FILE_OBJECT};
    use windows_sys::Win32::Security::{ACCESS_ALLOWED_ACE, ACL, DACL_SECURITY_INFORMATION, GetAce, GetSecurityDescriptorControl, SE_DACL_PROTECTED};
    let name = windows::wide(path.as_os_str());
    let (mut dacl, mut sd) = (std::ptr::null_mut::<ACL>(), std::ptr::null_mut());
    // SAFETY: `dacl` points into `sd`, which is freed with LocalFree after the last read.
    unsafe {
        match GetNamedSecurityInfoW(name.as_ptr(), SE_FILE_OBJECT, DACL_SECURITY_INFORMATION, std::ptr::null_mut(), std::ptr::null_mut(), &mut dacl, std::ptr::null_mut(), &mut sd) {
            0 => {}
            e => return Err(std::io::Error::from_raw_os_error(e as i32)),
        }
        let read = (|| {
            let (mut control, mut revision) = (0, 0);
            if GetSecurityDescriptorControl(sd, &mut control, &mut revision) == 0 {
                return Err(std::io::Error::last_os_error());
            }
            let mut aces = Vec::new();
            for i in 0..dacl.as_ref().map_or(0, |d| d.AceCount) {
                let mut ace = std::ptr::null_mut();
                if GetAce(dacl, i.into(), &mut ace) == 0 {
                    return Err(std::io::Error::last_os_error());
                }
                let ace = ace.cast::<ACCESS_ALLOWED_ACE>();
                let sid = windows::sid_string(std::ptr::addr_of_mut!((*ace).SidStart).cast())?;
                aces.push(((*ace).Header.AceType, (*ace).Header.AceFlags, sid));
            }
            Ok((control & SE_DACL_PROTECTED != 0, aces))
        })();
        LocalFree(sd);
        read
    }
}

#[cfg(windows)]
mod windows {
    /// `s` as a NUL-ended UTF-16 string.
    pub fn wide(s: &std::ffi::OsStr) -> Vec<u16> {
        use std::os::windows::ffi::OsStrExt;
        s.encode_wide().chain([0]).collect()
    }

    /// `sid` as a string (`S-1-5-…`).
    ///
    /// # Safety
    /// `sid` points to a valid SID.
    pub unsafe fn sid_string(sid: windows_sys::Win32::Security::PSID) -> std::io::Result<String> {
        use windows_sys::Win32::Foundation::LocalFree;
        use windows_sys::Win32::Security::Authorization::ConvertSidToStringSidW;
        let mut s = std::ptr::null_mut();
        // SAFETY: the caller's SID; the string Windows gives is NUL-ended and freed with LocalFree.
        unsafe {
            if ConvertSidToStringSidW(sid, &mut s) == 0 {
                return Err(std::io::Error::last_os_error());
            }
            let len = (0..).take_while(|&i| *s.add(i) != 0).count();
            let out = String::from_utf16_lossy(std::slice::from_raw_parts(s, len));
            LocalFree(s.cast());
            Ok(out)
        }
    }
}

/// Extract the web-client token embedded in a server-rendered workspace page.
pub fn extract_api_token(html: &str) -> Option<String> {
    let re = regex::Regex::new(r#""api_token":"(xoxc-[^"]+)""#).ok()?;
    re.captures(html).map(|c| c[1].to_string())
}

/// Fetch the `xoxc` token for a workspace using the `d` cookie, then confirm it with
/// `auth.test`.
pub async fn session_from_cookie(workspace_url: &str, cookie_d: &str) -> Result<WorkspaceSession> {
    session_from_cookie_with(&web_client()?, workspace_url, cookie_d).await
}

/// The client for Slack's web pages (a workspace's pages, the sign-in page): a browser's user agent.
/// One can be shared by many requests, as finding the sign-ins of every browser does. A connection is given
/// [`WEB_CONNECT_TIMEOUT`] and a request [`WEB_TIMEOUT`], as the code sign-in's client, so one that stalls does not
/// hang the cookie way or a script for ever (discovery's own 10 s per step comes first).
pub fn web_client() -> Result<reqwest::Client> {
    web_client_within(WEB_CONNECT_TIMEOUT, WEB_TIMEOUT)
}

/// How long [`web_client`] waits for a connection, and for a whole request.
const WEB_CONNECT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(15);
const WEB_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);

/// [`web_client`] waiting `connect` for a connection and `total` for a request.
fn web_client_within(connect: std::time::Duration, total: std::time::Duration) -> Result<reqwest::Client> {
    crate::api::http_client(|| reqwest::Client::builder().user_agent(USER_AGENT).redirect(crate::api::redirect_policy()).connect_timeout(connect).timeout(total))
}

/// [`session_from_cookie`] with the client `http`.
pub async fn session_from_cookie_with(http: &reqwest::Client, workspace_url: &str, cookie_d: &str) -> Result<WorkspaceSession> {
    let base = workspace_url.trim_end_matches('/');
    let html = http
        .get(format!("{base}/customize/emoji"))
        .header(reqwest::header::COOKIE, format!("d={cookie_d}"))
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;
    let token = extract_api_token(&html).ok_or_else(|| anyhow!("signed-out or unexpected page (no api_token) — is the session still valid?"))?;
    let resp: serde_json::Value = http
        .post(format!("{base}/api/auth.test"))
        .header(reqwest::header::COOKIE, format!("d={cookie_d}"))
        .form(&[("token", token.as_str())])
        .send()
        .await?
        .json()
        .await?;
    if resp["ok"].as_bool() != Some(true) {
        bail!("auth.test failed: {}", resp["error"].as_str().unwrap_or("unknown"));
    }
    let s = |k: &str| resp[k].as_str().unwrap_or_default().to_string();
    Ok(WorkspaceSession {
        team_id: s("team_id"),
        team_name: s("team"),
        url: s("url"),
        user_id: s("user_id"),
        user_name: s("user"),
        token,
        cookie_d: cookie_d.to_string(),
        source: None,
    })
}

/// A workspace a `d` cookie is signed in to, as Slack's sign-in page lists it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoggedInTeam {
    pub team_id: String,
    pub team_name: String,
    /// The workspace's address name: `acme` of `acme.slack.com`.
    pub domain: String,
    /// e.g. `https://acme.slack.com/`
    pub url: String,
    /// The email the account signs in with.
    pub email: String,
}

/// Slack's sign-in page; seen with a `d` cookie it lists every workspace that cookie is signed in to.
const SIGNIN_URL: &str = "https://slack.com/signin";

/// The workspaces Slack's sign-in page lists as signed in: its JSON (HTML-escaped inside the
/// page) has `"loggedInTeams":[{team_id, team_name, team_domain, user_email, …}]`. A signed-out or
/// unexpected page gives none.
pub fn parse_logged_in_teams(html: &str) -> Vec<LoggedInTeam> {
    find_logged_in_teams(html).unwrap_or_default()
}

/// Like [`parse_logged_in_teams`], but a page without the list (Slack changed its page) is `None`:
/// a signed-out page still has it, empty.
pub fn find_logged_in_teams(html: &str) -> Option<Vec<LoggedInTeam>> {
    const NAME: &str = "loggedInTeams";
    for (at, _) in html.match_indices(NAME) {
        let rest = unescape_html(&html[at + NAME.len()..]);
        // `":[…]` after the name (anything else, like `loggedInTeamsCount`, is not the list).
        let Some(list) = rest.trim_start_matches(['"', '\\']).trim_start().strip_prefix(':') else { continue };
        let list = list.trim_start();
        if !list.starts_with('[') {
            continue;
        }
        let Some(Ok(entries)) = serde_json::Deserializer::from_str(list).into_iter::<Vec<serde_json::Value>>().next() else { continue };
        let teams = entries
            .iter()
            .filter_map(|t| {
                let s = |k: &str| t[k].as_str().unwrap_or_default().to_string();
                let (team_id, domain) = (s("team_id"), s("team_domain"));
                if team_id.is_empty() || domain.is_empty() {
                    return None;
                }
                let url = team_address(&s("team_url")).unwrap_or_else(|| format!("https://{domain}.slack.com/"));
                Some(LoggedInTeam { team_id, team_name: s("team_name"), url, domain, email: s("user_email") })
            })
            .collect();
        return Some(teams);
    }
    None
}

/// A workspace's address from the `team_url` the sign-in page gives (`https://acme.slack.com/ssb/redirect` →
/// `https://acme.slack.com/`): only an https link whose host is a workspace's on Slack's domains, as
/// [`workspace_url`] takes them.
fn team_address(team_url: &str) -> Option<String> {
    let rest = team_url.trim().to_lowercase().strip_prefix("https://")?.to_string();
    let host = rest.split(['/', '?', '#']).next().unwrap_or_default();
    if !host.contains('.') {
        return None;
    }
    workspace_url(team_url).ok()
}

/// The address of the workspace typed as `input`: a name (`acme`), an address (`acme.slack.com`,
/// `acme.enterprise.slack.com`) or a link from the browser (`https://acme.slack.com/archives/…`).
pub fn workspace_url(input: &str) -> Result<String, String> {
    let text = input.trim().to_lowercase();
    let rest = text.strip_prefix("https://").or_else(|| text.strip_prefix("http://")).unwrap_or(&text);
    let host = rest.split(['/', '?', '#']).next().unwrap_or_default();
    // A pasted https link may carry its port.
    let host = host.strip_suffix(":443").unwrap_or(host);
    if host.is_empty() {
        return Err("type the workspace's name, like acme".into());
    }
    // Slack's own hosts that are not a workspace (and the sign-in page's).
    let (first, rest) = host.split_once('.').unwrap_or((host, ""));
    let slack = ["slack.com", "slack-gov.com"];
    if slack.contains(&host) || (slack.contains(&rest) && ["app", "www", "api", "edgeapi", "files", "status"].contains(&first)) || host.contains("slack-files") {
        return Err("that link doesn't name the workspace — type its name".into());
    }
    let host = if host.contains('.') { host.to_string() } else { format!("{host}.slack.com") };
    let name_ok = host.split('.').all(|part| !part.is_empty() && part.chars().all(|c| c.is_ascii_alphanumeric() || c == '-'));
    if !name_ok {
        return Err(format!("“{}” is not a workspace name — letters, digits and - only", input.trim()));
    }
    if !host.ends_with(".slack.com") && !host.ends_with(".slack-gov.com") {
        return Err(format!("{host} is not a Slack address — type the workspace's name, like acme"));
    }
    Ok(format!("https://{host}/"))
}

/// Undo the HTML escaping of an attribute value (`&quot;` and friends; `&amp;` last, so that an
/// escaped `&amp;quot;` stays `&quot;`).
fn unescape_html(text: &str) -> String {
    let mut out = text.to_string();
    for (from, to) in [("&quot;", "\""), ("&#34;", "\""), ("&#x22;", "\""), ("&#39;", "'"), ("&#x27;", "'"), ("&apos;", "'"), ("&lt;", "<"), ("&gt;", ">"), ("&#x2F;", "/"), ("&#47;", "/"), ("&amp;", "&")] {
        if out.contains(from) {
            out = out.replace(from, to);
        }
    }
    out
}

/// The workspaces the `d` cookie `cookie_d` is signed in to, from Slack's sign-in page.
pub async fn signed_in_teams(cookie_d: &str) -> Result<Vec<LoggedInTeam>> {
    Ok(signed_in_teams_with(&web_client()?, cookie_d).await?.unwrap_or_default())
}

/// [`signed_in_teams`] with the client `http`; `None` when the page has no list of workspaces at
/// all (see [`find_logged_in_teams`]).
pub async fn signed_in_teams_with(http: &reqwest::Client, cookie_d: &str) -> Result<Option<Vec<LoggedInTeam>>> {
    let html = http.get(SIGNIN_URL).header(reqwest::header::COOKIE, format!("d={cookie_d}")).send().await?.error_for_status()?.text().await?;
    Ok(find_logged_in_teams(&html))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn a_web_page_that_never_answers_is_given_up() {
        // As the code sign-in's client: a stalled connection does not hang the cookie way or a script for ever.
        assert_eq!((WEB_CONNECT_TIMEOUT, WEB_TIMEOUT), (std::time::Duration::from_secs(15), std::time::Duration::from_secs(30)));
        let _ = rustls::crypto::ring::default_provider().install_default();
        // A server that takes the connection and never answers.
        let silent = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let http = web_client_within(std::time::Duration::from_millis(300), std::time::Duration::from_millis(300)).unwrap();
        let started = std::time::Instant::now();
        let r = session_from_cookie_with(&http, &format!("http://{}/", silent.local_addr().unwrap()), "x").await;
        assert!(r.is_err());
        assert!(started.elapsed() < std::time::Duration::from_secs(5), "{:?}", started.elapsed());
    }

    #[test]
    fn an_old_window_session_still_loads() {
        let source: SessionSource = serde_json::from_str(r#"{"kind":"window"}"#).unwrap();
        assert_eq!(source, SessionSource::Window);
    }

    #[test]
    fn extracts_token() {
        let html = r#"<script>var boot = {"api_token":"xoxc-123-456-abc","x":1}</script>"#;
        assert_eq!(extract_api_token(html).as_deref(), Some("xoxc-123-456-abc"));
        assert_eq!(extract_api_token("<html>signin</html>"), None);
    }

    #[test]
    fn debug_redacts_secrets() {
        let ws = WorkspaceSession {
            team_id: "T1".into(),
            team_name: "t".into(),
            url: "https://t.slack.com/".into(),
            user_id: "U1".into(),
            user_name: "u".into(),
            token: "xoxc-secret".into(),
            cookie_d: "cookie-secret".into(),
            source: None,
        };
        let dbg = format!("{ws:?}");
        assert!(!dbg.contains("secret"));
        assert_eq!(ws.api_base(), "https://t.slack.com/api/");
    }

    fn account(team: &str, name: &str, user: &str) -> WorkspaceSession {
        WorkspaceSession { team_id: team.into(), team_name: name.into(), user_id: user.into(), user_name: format!("person-{user}"), ..Default::default() }
    }

    #[test]
    fn two_accounts_of_one_team_are_kept() {
        let mut p = Profile::default();
        p.upsert(account("T0000000001", "acme", "U0000000001"));
        p.upsert(account("T0000000001", "acme", "U0000000002"));
        assert_eq!(p.workspaces.len(), 2);
        assert_eq!(p.default_team.as_deref(), Some("T0000000001"));
        // The first saved is the one started.
        assert_eq!(p.workspace(None).unwrap().user_id, "U0000000001");
    }

    #[test]
    fn the_same_account_is_replaced() {
        let mut p = Profile::default();
        p.upsert(account("T0000000001", "acme", "U0000000001"));
        p.upsert(account("T0000000002", "other", "U0000000001"));
        p.upsert(WorkspaceSession { source: Some(SessionSource::Cookie), ..account("T0000000001", "acme", "U0000000001") });
        assert_eq!(p.workspaces.len(), 2);
        let acme = p.matching("acme");
        assert_eq!(acme.len(), 1);
        assert_eq!(acme[0].source, Some(SessionSource::Cookie));
        // Forgetting an account touches only that team + user.
        assert!(!p.remove("T0000000001", "U0000000009"));
        assert!(p.remove("T0000000001", "U0000000001"));
        assert_eq!(p.workspaces.len(), 1);
        // The default no longer names a team without accounts.
        assert_eq!(p.default_team.as_deref(), Some("T0000000002"));
        assert!(p.remove("T0000000002", "U0000000001"));
        assert_eq!(p.default_team, None);
    }

    #[test]
    fn signing_in_again_keeps_the_order() {
        let mut p = Profile::default();
        p.upsert(account("T0000000001", "acme", "U0000000001"));
        p.upsert(account("T0000000001", "acme", "U0000000002"));
        p.upsert(account("T0000000002", "other", "U0000000001"));
        p.upsert(WorkspaceSession { source: Some(SessionSource::Cookie), ..account("T0000000001", "acme", "U0000000001") });
        let order: Vec<(&str, &str)> = p.workspaces.iter().map(|w| (w.team_id.as_str(), w.user_id.as_str())).collect();
        assert_eq!(order, [("T0000000001", "U0000000001"), ("T0000000001", "U0000000002"), ("T0000000002", "U0000000001")]);
        assert_eq!(p.workspace(None).unwrap().user_id, "U0000000001");
        assert_eq!(p.workspaces[0].source, Some(SessionSource::Cookie));
    }

    #[test]
    fn a_broken_session_file_is_not_replaced() {
        let dir = std::env::temp_dir().join(format!("ttyak-session-read-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        // No file: a new, empty profile.
        assert!(Profile::read_or_new(&dir.join("session.json")).unwrap().workspaces.is_empty());
        std::fs::write(dir.join("session.json"), "{ not json").unwrap();
        assert!(Profile::read_or_new(&dir.join("session.json")).is_err());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn old_session_files_still_load() {
        let text = r#"{"workspaces":[
            {"team_id":"T0000000001","team_name":"acme","url":"https://acme.slack.com/","user_id":"U0000000001","user_name":"one","token":"xoxc-made-up","cookie_d":"made-up"},
            {"team_id":"T0000000002","team_name":"other","url":"https://other.slack.com/","user_id":"U0000000001","user_name":"one","token":"xoxc-made-up","cookie_d":"made-up"},
            {"team_id":"T0000000001","team_name":"acme","url":"https://acme.slack.com/","user_id":"U0000000002","user_name":"two","token":"xoxc-made-up","cookie_d":"made-up"}
        ],"default_team":"T0000000001"}"#;
        let p: Profile = serde_json::from_str(text).unwrap();
        assert!(p.workspaces.iter().all(|w| w.source.is_none()));
        let start = p.workspace(None).unwrap();
        assert_eq!((start.team_id.as_str(), start.user_id.as_str()), ("T0000000001", "U0000000001"));
        let users: Vec<&str> = p.matching("acme").iter().map(|w| w.user_id.as_str()).collect();
        assert_eq!(users, ["U0000000001", "U0000000002"]);
        assert_eq!(p.matching("T0000000002").len(), 1);
        assert!(p.matching("nope").is_empty());
        // Saving it again writes no source for the old ones.
        assert!(!serde_json::to_string(&p).unwrap().contains("source"));
    }

    #[test]
    fn source_round_trips() {
        let sources = [
            SessionSource::Browser { browser: "chrome".into(), profile: PathBuf::from("/home/person/.config/google-chrome/Profile 1"), label: "Chrome · Work".into() },
            SessionSource::Window,
            SessionSource::Cookie,
            SessionSource::Code { email: "person@example.com".into() },
        ];
        for source in sources {
            let ws = WorkspaceSession { source: Some(source.clone()), ..account("T0000000001", "acme", "U0000000001") };
            let text = serde_json::to_string(&ws).unwrap();
            let back: WorkspaceSession = serde_json::from_str(&text).unwrap();
            assert_eq!(back.source, Some(source), "{text}");
        }
        let text = serde_json::to_string(&WorkspaceSession { source: Some(SessionSource::Window), ..Default::default() }).unwrap();
        assert!(text.contains(r#""source":{"kind":"window"}"#), "{text}");
        let text = serde_json::to_string(&WorkspaceSession { source: Some(SessionSource::Code { email: "person@example.com".into() }), ..Default::default() }).unwrap();
        assert!(text.contains(r#""kind":"code""#), "{text}");
    }

    #[test]
    fn the_signin_page_names_every_workspace() {
        let teams = parse_logged_in_teams(include_str!("../tests/fixtures/signin-page.html"));
        let acme = LoggedInTeam { team_id: "T0000000001".into(), team_name: "Acme & Sons".into(), domain: "acme".into(), url: "https://acme.slack.com/".into(), email: "person@example.com".into() };
        let widgets = LoggedInTeam { team_id: "T0000000002".into(), team_name: "Widgets Inc".into(), domain: "widgets-inc".into(), url: "https://widgets-inc.slack.com/".into(), email: "person@example.com".into() };
        assert_eq!(teams, [acme, widgets]);
    }

    #[test]
    fn a_team_s_address_comes_from_its_team_url() {
        let page = |team_url: Option<&str>| {
            let field = team_url.map_or(String::new(), |u| format!(r#""team_url":"{u}","#));
            format!(r#"{{"loggedInTeams":[{{"team_id":"T0000000001","team_name":"Acme",{field}"team_domain":"acme","user_email":"person@example.com"}}]}}"#)
        };
        let url = |team_url: Option<&str>| parse_logged_in_teams(&page(team_url))[0].url.clone();
        // Enterprise Grid: the host is not `<domain>.slack.com`; the path (and JSON's escaped slashes) is dropped.
        assert_eq!(url(Some("https://acme-corp.enterprise.slack.com/ssb/redirect")), "https://acme-corp.enterprise.slack.com/");
        assert_eq!(url(Some(r"https:\/\/acme.slack-gov.com\/ssb\/redirect")), "https://acme.slack-gov.com/");
        // None, not https, not a workspace on Slack's hosts: the domain's.
        for other in [None, Some(""), Some("http://acme2.slack.com/"), Some("https://evil.example.com/"), Some("https://app.slack.com/client"), Some("https://acme2/"), Some("acme2.slack.com")] {
            assert_eq!(url(other), "https://acme.slack.com/", "{other:?}");
        }
    }

    /// A folder removed with what it holds when the test ends, a failed assertion too.
    struct Temp(PathBuf);
    impl Drop for Temp {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn owner_only_sddl_names_the_user_system_and_admins() {
        let sid = "S-1-5-21-1-2-3-1001";
        assert_eq!(owner_only_sddl(sid, false), "D:P(A;;FA;;;S-1-5-21-1-2-3-1001)(A;;FA;;;SY)(A;;FA;;;BA)");
        assert_eq!(owner_only_sddl(sid, true), "D:P(A;OICI;FA;;;S-1-5-21-1-2-3-1001)(A;OICI;FA;;;SY)(A;OICI;FA;;;BA)");
    }

    /// A file system without ACLs (FAT, some network shares) does not cost the write: the file is still written whole.
    /// The ACL goes on the empty temporary file, before the secret is in it.
    #[test]
    fn write_private_survives_an_acl_failure() {
        let dir = Temp(std::env::temp_dir().join(format!("ttyak-session-acl-{}", std::process::id())));
        std::fs::create_dir_all(&dir.0).unwrap();
        let path = dir.0.join("session.json");
        write_private_with(&path, b"{\"secret\":1}", |p, dir| {
            assert!(!dir && p.file_name().unwrap().to_string_lossy().ends_with(".tmp"), "{p:?}");
            assert_eq!(std::fs::read(p)?.len(), 0, "still empty");
            Err(std::io::ErrorKind::Unsupported.into())
        })
        .unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"{\"secret\":1}");
        let names: Vec<_> = std::fs::read_dir(&dir.0).unwrap().map(|e| e.unwrap().file_name()).collect();
        assert_eq!(names, ["session.json"], "no temporary file left");
    }

    #[cfg(windows)]
    #[test]
    fn write_private_is_owner_only_on_windows() {
        let dir = Temp(std::env::temp_dir().join(format!("ttyak-session-dacl-{}", std::process::id())));
        std::fs::create_dir_all(&dir.0).unwrap();
        let path = dir.0.join("session.json");
        let user = user_sid().unwrap();
        // Written new, then again over the old one.
        for _ in 0..2 {
            write_private(&path, b"{}").unwrap();
            let (protected, aces) = dacl_of(&path).unwrap();
            assert!(protected, "inherits nothing");
            assert_eq!(aces.len(), 3, "{aces:?}");
            assert_eq!(aces[0], (ACCESS_ALLOWED, 0, user.clone()), "{aces:?}");
        }
    }

    /// Two writers at once (two profiles' ttyaks writing `updates.json`) never mix their files: each write goes through
    /// a temporary file of its own, so the file is always one whole write and nothing is left beside it.
    #[test]
    fn writers_at_once_never_mix_their_files() {
        let dir = Temp(std::env::temp_dir().join(format!("ttyak-session-writers-{}", std::process::id())));
        std::fs::create_dir_all(&dir.0).unwrap();
        let path = dir.0.join("updates.json");
        let whole = |b: &[u8]| b.len() == 1 << 16 && b.iter().all(|c| *c == b[0]);
        let writers: Vec<_> = (b'a'..=b'h')
            .map(|c| {
                let path = path.clone();
                std::thread::spawn(move || (0..40).map(|_| write_private(&path, &[c; 1 << 16]).map_err(|e| format!("{e:#}"))).collect::<Result<Vec<()>, String>>())
            })
            .collect();
        // (All joined before any assertion: a writer still going would put files back as the folder is removed.)
        let results: Vec<_> = writers.into_iter().map(|w| w.join().unwrap()).collect();
        for r in results {
            r.expect("every write is written");
        }
        assert!(whole(&std::fs::read(&path).unwrap()), "one whole write");
        let names: Vec<_> = std::fs::read_dir(&dir.0).unwrap().map(|e| e.unwrap().file_name()).collect();
        assert_eq!(names, ["updates.json"], "no temporary file left");
    }

    #[test]
    fn a_broken_session_file_is_named() {
        let dir = std::env::temp_dir().join(format!("ttyak-session-load-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("session.json");
        std::fs::write(&path, "{ 7 }").unwrap();
        let e = format!("{:#}", Profile::load_at("work", &path).unwrap_err());
        assert!(e.contains(&path.display().to_string()), "{e}");
        std::fs::remove_file(&path).unwrap();
        let e = format!("{:#}", Profile::load_at("work", &path).unwrap_err());
        assert!(e.contains("no session for profile 'work'"), "{e}");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn workspace_url_takes_names_addresses_and_links() {
        for typed in ["acme", "acme.slack.com", "https://acme.slack.com/archives/C1", " Acme ", "ACME.slack.com/", "http://acme.slack.com"] {
            assert_eq!(workspace_url(typed).as_deref(), Ok("https://acme.slack.com/"), "{typed:?}");
        }
        assert_eq!(workspace_url("acme.enterprise.slack.com").as_deref(), Ok("https://acme.enterprise.slack.com/"));
        assert_eq!(workspace_url("my-team2").as_deref(), Ok("https://my-team2.slack.com/"));
        // The port of a pasted https link is dropped.
        assert_eq!(workspace_url("https://acme.slack.com:443/archives/C1").as_deref(), Ok("https://acme.slack.com/"));
    }

    #[test]
    fn workspace_url_refuses_what_names_no_workspace() {
        assert_eq!(workspace_url("https://app.slack.com/client/T1/C1"), Err("that link doesn't name the workspace — type its name".to_string()));
        assert!(workspace_url("app.slack.com").is_err());
        // Slack's own hosts that are not a workspace.
        for host in ["files.slack.com", "https://status.slack.com/", "api.slack.com/methods", "edgeapi.slack.com", "www.slack.com", "https://slack-files.com/T1-F1-abc", "app.slack-gov.com"] {
            assert_eq!(workspace_url(host), Err("that link doesn't name the workspace — type its name".to_string()), "{host:?}");
        }
        assert!(workspace_url("acme.slack.com:8080").is_err());
        for bad in ["", "   ", "a b", "acme.example.com", "acme..slack.com", "ac_me"] {
            assert!(workspace_url(bad).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn a_page_without_the_list_is_not_a_signed_out_page() {
        // Signed out (checked 2026-10-20): the list is there, empty.
        assert_eq!(find_logged_in_teams(r#"<div data-props="{&quot;loggedInTeams&quot;:[],&quot;x&quot;:1}"></div>"#), Some(vec![]));
        assert_eq!(find_logged_in_teams(include_str!("../tests/fixtures/signin-page.html")).map(|t| t.len()), Some(2));
        // No list at all, or one that cannot be read: Slack changed its page.
        assert_eq!(find_logged_in_teams("<html><title>Sign in | Slack</title></html>"), None);
        assert_eq!(find_logged_in_teams("loggedInTeams&quot;:[{&quot;team_id&quot;:&quot;T0000000001"), None);
        assert_eq!(find_logged_in_teams(r#"{"loggedInTeamsCount":2}"#), None);
    }

    #[test]
    fn a_page_without_teams_gives_none() {
        // Signed out: an empty list, or none at all.
        assert!(parse_logged_in_teams(r#"<div data-props="{&quot;loggedInTeams&quot;:[],&quot;x&quot;:1}"></div>"#).is_empty());
        assert!(parse_logged_in_teams("<html><title>Sign in | Slack</title></html>").is_empty());
        // Garbage: a cut-off list, not a list, entries without an id or domain.
        assert!(parse_logged_in_teams("loggedInTeams&quot;:[{&quot;team_id&quot;:&quot;T0000000001").is_empty());
        assert!(parse_logged_in_teams(r#"{"loggedInTeams":{"team_id":"T0000000001"}}"#).is_empty());
        assert!(parse_logged_in_teams(r#"{"loggedInTeams":[{"team_name":"x"},{"team_id":"T0000000001"},7]}"#).is_empty());
        assert!(parse_logged_in_teams("loggedInTeams").is_empty());
    }
}
