## Read this first

ttyak is not an official Slack app, and it is not a desktop app with windows of its own. So signing in is not as
smooth as in Slack's own apps. Here is why:

- Slack's own sign-in ends by handing the session to the Slack app through an "open in Slack" link (a `slack://`
  link). ttyak is not that app and cannot take such links.
- Slack has no sign-in for apps like ttyak. ttyak talks to Slack the way the web client in your browser does, with a
  browser session: an `xoxc-…` token plus the `d` cookie.

So ttyak gets a browser session in one of three ways:

- **With a code from your email:** you ask Slack for a code on your phone or in any browser, and type the code into
  ttyak. This works over SSH and on machines with no browser.
- **In your browser:** you sign in to Slack in a browser on the machine ttyak runs on, any way Slack offers, and ttyak
  picks the new session up from the browser's files.
- **Paste a cookie:** you copy the `d` cookie from a browser's developer tools and paste it.

Workspaces that require SSO or two-factor sign-in can't use the email code: ttyak shows them dim ("needs your
browser"). Sign in to those in your browser (not tried yet with such a workspace).

## The guide

`ttyak auth login` (and plain `ttyak` on a profile with no sign-ins yet, in a terminal) guides you, right in the
terminal. It looks for Slack in your browsers and lists every workspace each browser profile is signed in to, grouped
by where it comes from (`Chrome · Work  you@example.com`, `Firefox · default-release`), with who you are there. Below
them come the three ways to sign in afresh, for any workspace.

- ↑↓ or j k move, Space marks several, Enter takes the marked ones (or the one under the cursor), `/` filters, a click
  marks a row. Esc in the list leaves it; nothing more is saved.
- A workspace a browser is signed out of is shown dim with the reason (`signed out in Firefox · default-release`), and
  so is a browser profile ttyak could not look at. A workspace already in the profile says `already signed in`
  (picking it refreshes the session).
- The marked browser sign-ins are saved first, then each marked way runs; Esc in a way goes back to the list.
- Each workspace ends in one line: `✓ Signed in to Acme as you — profile default saved.` (or `✗ Acme: <why>`).

Nothing is signed out anywhere, and ttyak never opens a browser itself. It reads Firefox everywhere, and Chrome,
Chromium, Edge and Brave on Linux and macOS (not on Windows: see
[From the browser you already use](#from-the-browser-you-already-use)). The sign-ins go to the profile `default`
unless you name another with `--profile NAME`: see [Profiles and sessions](Profiles-and-sessions).

## With a code from your email

1. ttyak asks for your email address, then shows a QR code and a link (`app.slack.com/get-started?email=…`, your
   address filled in) for your phone or any browser.
2. There, Slack sends you a code by email. This step needs Slack's captcha, so only a real browser can do it. Don't
   enter the code on the phone.
3. Type the code into ttyak's `Code: ___-___` prompt (a dash or a space you type doesn't matter). ttyak confirms it
   with Slack itself and lists the workspaces of that address. A wrong or expired code asks again (the phone can ask
   for a new one); after too many tries Slack makes you wait.
4. Mark the workspaces to sign in to (when only one can be used, it is marked already) and press Enter.

Workspaces that require SSO or two-factor are shown dim, `needs your browser (SSO)` or `needs your browser
(two-factor)`. Slack's list says which ones they are, and ttyak can't sign in to them with the code: use
[your browser](#in-your-browser) for those (not tried yet: no such workspace to test with). When every workspace of
the address is like that, ttyak says so for each and shows no list.

In a small terminal (80 × 24) the QR code is drawn smaller; in a smaller one only the link is shown. This way works
over SSH and on machines with no browser.

`ttyak auth login --email you@example.com` goes straight to it with the address filled in.

## In your browser

ttyak prints a link: Slack's sign-in page (with `--workspace`, that workspace's address). Open it in whichever browser
and profile you like and sign in there, any way Slack offers. ttyak watches the browser profiles (read only: it looks
at their cookie files every 2 seconds) and, when one gets a new Slack session, offers it, marked: Enter saves it. It
can take the browser a little while to save the new session to its files. Esc stops waiting; after 10 minutes with
nothing new ttyak gives up.

While it waits, ttyak names each browser it can't read: on Windows, Chrome (app-bound encryption); elsewhere, a browser
of Chrome's family whose key the system keyring or the Keychain did not give. When it can read none, the row is dim and
says why (`no browser found here — use the code`, or which browser and why), pointing to Firefox or the code.

This way needs a browser on the machine ttyak runs on. Over SSH, use the code instead, or reach that machine's
browser through a remote desktop (such as RDP) while ttyak runs in the SSH session.

## Paste a cookie

ttyak asks for the value of the `d` cookie for `slack.com`, from a browser where you are signed in (its developer
tools → Application → Cookies). What you paste is not shown, only how many characters; a leading `d=` is dropped.
Slack then lists the workspaces that cookie is signed in to: mark them and press Enter. A cookie that is signed in
nowhere is asked for again.

## One workspace

`ttyak auth login --workspace acme.slack.com` (or `acme`, or a link from the browser) in a terminal lists the browser
profiles signed in to that workspace, then a code from your email, your browser and a pasted cookie. That workspace is
marked in each way's list, and the browser way's link is the workspace's own address.

## From the browser you already use

`ttyak auth login --workspace acme.slack.com --from-browser firefox` (or `chrome`) takes the session from a browser
you are signed in to Slack with, without the guide.

- `chrome` stands for the whole family: Chrome, Chromium, Edge and Brave. On Linux ttyak also finds the snap and
  flatpak Firefox, and the flatpak Chrome and Chromium.
- The browser's cookie database is copied to a temporary folder (yours only) and read there, never changed; the copy is
  deleted afterwards.
- Chrome's family encrypts its cookies: ttyak decrypts them with the browser's key from the system keyring (Linux; a
  locked keyring is asked to unlock, not tried yet) or the Keychain (macOS). On macOS the Keychain asks once per
  browser, one after another (the guide too); a question left unanswered for a minute counts as no key. Answer it with
  “Allow”, not “Always Allow”: that would let any program read the key through the `security` tool without asking.
  (The Keychain question itself has not been tried yet with a real Chrome key.)
- `--browser-profile DIR` reads one profile (or a browser's folder of profiles); without it every profile of that
  browser is tried.
- When several profiles sign in to the workspace as different people, ttyak lists them (the browser profile's name
  and who you are there) and asks which; without a terminal to ask in it takes the one used most recently.
- On Windows ttyak reads no browser of Chrome's family: Chrome encrypts its cookies there so that only Chrome can read
  them (app-bound encryption), and `--from-browser chrome` says so. Firefox can be read.

## For scripts

Without a terminal ttyak asks nothing:

- `ttyak auth login --workspace acme.slack.com` reads the value of the `d` cookie for `slack.com` from standard input
  (sign in in any browser, then devtools → Application → Cookies). `TTYAK_COOKIE_D` or `--cookie VALUE` give it
  directly (`--cookie` ends up in your shell history: prefer the variable or standard input). Either way ttyak fetches
  the token itself and checks it with `auth.test`.
- `--from-browser` works without a terminal too: it takes the profile used most recently.
- `ttyak auth login --email you@example.com` prints the link (on standard error), reads the code from standard input
  and signs in to every listed workspace that can be used; it fails when none was.
- Without `--workspace` or `--email` it stops: `which workspace? add --workspace acme.slack.com`.

In a terminal `--cookie`, `TTYAK_COOKIE_D` and `--from-browser` work the same and skip the guide (without
`--workspace`, ttyak asks for the workspace). `--email` can't be combined with `--workspace`, `--cookie` or
`--from-browser`.

## If it doesn't work

- [Troubleshooting](Troubleshooting)
