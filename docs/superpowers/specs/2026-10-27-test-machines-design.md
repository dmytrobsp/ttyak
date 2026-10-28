# Test machines and shared resources — design (2026-10-27)

Issue: [#30](https://github.com/dmytrobsp/ttyak/issues/30). Machine-specific facts (host aliases, VM names, account
names, paths) are not in this file: they live in the git-ignored local files (`AGENTS.local.md` and the other
projects' equivalents) and in the private repository below.

## Intent

Live checks run on the maintainer's own Linux desktop and on two shared boxes: a Windows VM on that desktop and a
macOS laptop. Two things go wrong:

- **Collisions.** The boxes are shared by the maintainer and by agents of several projects. The maintainer's words:
  "once or twice we've been running manual tests when a sub-agent just intruded and started doing its own thing."
  Nothing on a box says it is in use.
- **No isolation.** Test windows and notifications on the desktop get in the maintainer's way; the Windows VM is also
  the maintainer's work VM; there is one of each box, so agents queue for it or step on each other.

The maintainer: "whoever uses a VM should claim it. Then others who want to use it stay in queue. Pretty much what
mutex does. Either that, or we find a way to spin up cheap Windows instances" — and then: "the entire approach should
be designed with throwaway instances in mind."

**Done means:** every use of a shared box goes through a claim (agents and the maintainer alike); an agent that needs a
Linux, Windows or macOS environment for a test can get a fresh throwaway one, use it, and have it gone afterwards; a
ttyak Slack test account is claimed the same way before use; nothing of this appears on the maintainer's screen.

## Goals

1. **Box claims** — a mutex with a queue for each shared box, usable by every project and every host.
2. **Throwaway instances** — Linux containers with a desktop inside, Windows linked clones of a clean base, one macOS
   VM on the Mac; made per test, removed after; capacity claimed like a box.
3. **ttyak account claims** — a ledger of the Slack test accounts and their sessions, with a claim per account; the
   session copied into the instance that runs the test.
4. **ttyak's agent docs** use all of the above: live checks run in instances by default; the desktop's hidden test
   workspace only for what an instance cannot do.

## Non-goals

- A central coordinator, a server or a daemon on the desktop that other hosts depend on ("#2 complicates things too
  much"). Each lock lives with the thing it guards.
- Rented cloud Macs ("too expensive") or a macOS VM on the Linux desktop (Apple's licence allows macOS only on Apple
  hardware). A second Mac, if one is ever bought, is just another box.
- Account claims for projects other than ttyak.
- Gating ssh itself (a forced command or login check on the boxes): enforcement lives in the tool.
- Anything for other maintainers: the box tool and its instructions are machine-specific; only the ttyak account-claim
  tool's code is public.

## Anti-goals (a failure even if it works)

- A claim that outlives a dead agent and blocks the box until someone notices.
- An agent that waits silently forever with nobody told why.
- Throwaway instances that wear the SSD: rebuilding images per test, compiling inside instances, writing test scratch
  to disk.
- Anything on the maintainer's screen; any capture of the real microphone, camera or screen.
- Copies of the maintainer's work VM (its work data, its retail licence) used as test instances.
- Tokens printed, committed, or left behind in a removed instance.

## Constraints

- The hard rules in [AGENTS.md](../../../AGENTS.md) hold everywhere (bash, `set -euo pipefail`, no recursive
  rm/chmod on variable paths, the repository's own git identity in every new repository, public wording, no capture).
- A claim wait is not a live observation: the "about 10 minutes" bound on live waits applies to watching ttyak, not to
  waiting in a claim queue (below, "Waiting").
- Hardware: the desktop has 60 GB RAM, 32 threads, KVM, a few hundred GB of free SSD. The Windows VM is set to 10 GB
  from its next start (done 2026-10-27 on the maintainer's word; it ran with 24 GB). The Mac has 18 GB.
- The shared Windows VM has a retail licence and BitLocker with its virtual TPM: it is never cloned.
- Agents run on the desktop, and sometimes on the Mac or inside Windows (those also use the boxes, to test or
  compile). All ttyak Slack work runs from the desktop.

## Part 1 — the private `<tool>` repository

- A new private GitHub repository (the maintainer's account), cloned on the desktop under `~/projects/`, and on the Mac
  and in Windows where agents run there. It commits with the maintainer's GitHub identity, set repo-locally.
- It holds: the claim tool, the instance tools, the Linux images' recipes, the scripts that build and refresh the
  Windows and macOS bases, and short instructions. Each project's git-ignored local file points to it and lists the
  boxes; no project repository repeats the instructions.
- Its README may name the boxes and hosts (it is private); still no tokens or passwords in it.
- **Access from the boxes**: each machine that clones it (the Mac, the shared Windows VM, later boxes) gets its own
  GitHub key for this repository only (the maintainer's decision). My call: read-only deploy keys; changes to the tool
  are made from the desktop, so a box never holds a key that can write.

## Part 2 — box claims

What the maintainer decided:

- **Everyone claims.** The maintainer rarely uses a box alone ("almost always with an agent or via an agent"); when
  they do — their own work in the shared Windows VM included — they claim it too, with the same command, from the
  desktop or from inside the box. Once Windows clones exist, agents use clones by default and claim the shared
  Windows VM only when a test needs that VM itself.
- **A claim says who and why**: holder (project, agent or person) and purpose. Others wait in arrival order (a queue)
  and get the box when it is released.
- **The lock lives on the box itself.** An agent on the desktop claims over ssh; an agent on the box claims locally;
  the meaning is the same. There is no central host.
- **Waiting is the agent's choice**: with a time limit or without one (an autonomous agent may have nobody to report
  to). Optionally the tool reports at an interval the agent chooses — the holder, purpose, since when, last renewal,
  and the time waited so far — so the agent can wake up and decide: keep waiting, do something else, or look into a
  claim that seems frozen.
- **On the box**: leave it as found, unless the goal was to change the box's state.
- **Enforced by the tool**, not by ssh: ssh itself does not refuse connections, but work on a box goes through the
  tool (its "run under a claim" command, its file copies, its interactive shell), and the tool refuses without a live
  claim held by the caller. The agent docs require using the tool for every box.
- **The box list** is the tool's configuration: adding a box (another Mac, a second Windows VM) is one entry plus the
  box-side setup, with no change to the tool. A box can be **withdrawn** from the list (e.g. the maintainer takes the
  Mac back for work): claims on it are then refused with that reason; withdrawing waits for the current holder, or
  is done as a claim. The maintainer can also simply claim a box for as long as they need it.

My calls (the maintainer may override):

- **Leases.** The holder renews its claim while it works (the tool can renew in the background for a long command); a
  claim not renewed within its lease time is shown as stale. A stale claim can be broken by whoever is next in line;
  the holder or the maintainer can release any claim at any time. Every break and forced release is logged on the box.
- **Commands** (names are the builder's): claim (with wait options), release, renew, status (holder, queue, lease
  state), and a way to run one command under a claim (claim → run → release, also on failure).
- **A stopped VM**: whoever needs it starts it from the desktop, then claims as usual; the claim tool does not start or
  stop VMs. A box that cannot be reached cannot be claimed; the tool says so.

Left to the builder: the lock primitive on each OS (it must be atomic over ssh and locally, on Linux, macOS and
Windows — e.g. an atomic directory create plus files, with PowerShell on Windows); the queue's storage; lease times;
how a waiting agent gets its periodic reports (printed lines it can watch, given how agents run commands).

## Part 3 — throwaway instances

Common rules:

- **Made per test, removed after.** Each instance is labelled with its holder and purpose. An instance whose claim has
  gone stale is found and removed (an agent's crash leaves nothing behind for long).
- **Capacity is a claim.** Instances on a host draw from slots counted from that host's free RAM (not a fixed number);
  an agent that finds no slot queues like for a box, with the same wait options and reports. The host that runs the
  instances (the desktop, the Mac) holds that lock.
- **A RAM reserve** (my call): each host keeps a configurable amount free for the maintainer and never hands it out
  (to start with: 16 GB on the desktop, plus the shared Windows VM's 10 GB even while it is stopped, since it may be
  started at any time). Free RAM is read when an instance is made; an instance that would cut into the reserve
  waits for a slot.
- **The macOS VM and the Mac** (my call): using the macOS VM takes the Mac's VM slot, not the Mac box claim, so the Mac
  itself stays usable while the VM runs (within the reserve).
- **SSD.** Images and bases are built once, and again only when they change. An instance adds only a thin
  copy-on-write layer. Instance scratch (fake home, logs) goes to RAM (tmpfs) where possible. Programs under test are
  built on the host and mounted read-only; nothing is compiled inside an instance.

**Linux** (rootless podman on the desktop):

- An image with a headless desktop inside: a compositor with an invisible output, D-Bus, a notification daemon,
  PipeWire with a null sink, a terminal. A spike on 2026-10-27 showed this works with headless sway in a rootless
  container: notifications reached the daemon, a null sink came up, terminal windows opened and took focus by command;
  sway needed `--cap-add=SYS_NICE`. Window raising by ttyak itself was not tried.
- **A choice of desktop**: Hyprland first (the maintainer's own), sway, and GNOME and KDE if they run headless in
  rootless podman. ttyak's notifications and window raise differ per desktop; today only the maintainer's can be
  tested.
- Agents drive and observe it from the desktop (the compositor's IPC, the notification daemon's list, the null sink's
  recordings, screenshots of the invisible output).

**Windows** (libvirt on the desktop):

- A clean **base** from Microsoft's free Windows 11 Enterprise evaluation (90 days, meant for testing): no BitLocker,
  nothing of the maintainer's, Windows Update and indexing off, ssh and the test tools installed, automatic log-on of
  a test user. Built by a script, and rebuilt by that script when the evaluation runs out. A short manual step by the
  maintainer at each rebuild is acceptable (e.g. downloading the ISO, which sits behind a registration form); the
  script says exactly what to do and does everything else. The tool warns ahead of the evaluation's end.
- Each test gets a **linked clone** (a qcow2 overlay on the frozen base) with its own fresh virtual TPM and NVRAM;
  removed after.
- The shared Windows VM stays as it is: the maintainer's work VM and a claimed box, never cloned.
- Possible later: start clones from a saved, already-booted memory state to cut the boot wait (the builder may try it).

**macOS** (on the Mac):

- The Mac (the maintainer's work laptop, rarely used for work now) stays a claimed box and hosts **one** throwaway
  macOS VM at a time (Apple's licence allows two VMs per Mac; 18 GB allows one), with a VM tool such as Tart.
- The base is built from **Apple's own restore image** (not a third-party prebuilt image), by a script; short manual
  steps by the maintainer are acceptable where macOS requires a click (the setup assistant, the permission for the
  terminal to show notifications), and the script says exactly what to do.
- A dedicated Mac mini may be added later (the maintainer is considering an M2 with 16 GB / 256 GB). It joins as
  another box and becomes the macOS VM host (one or two VMs); the laptop then stays a box only. Nothing in the
  design depends on it.

## Part 4 — ttyak test accounts (ttyak only, desktop only)

- **The ledger**: a git-ignored file in the ttyak checkout listing each Slack test account — its name, its profile,
  what it is for, and its permissions (e.g. DND may be changed and must be restored; never sign out). The account
  details now in `AGENTS.local.md` move there; `AGENTS.local.md` points to it.
- **Sessions**: the ledger's sessions, owner-only (0600) files next to it, never printed, become the master copies.
  Today they live in the maintainer's real ttyak config folder; the builder moves or copies them with the maintainer's
  word.
- **Claims**: one claim per account, before any test that uses it — the fix for "one test per Slack account at a
  time". A local lock in the ttyak checkout (one host, so simpler than box claims), with holder, purpose, a queue, the
  same wait options and reports, and stale handling. Its code lives in ttyak's `tools/` (public, nothing private in
  it).
- **Into instances**: a test copies the claimed account's session into the instance's fake home; removing the
  instance removes the copy. The desktop always does the copying — into Linux containers and Windows clones on the
  desktop, and over ssh into the macOS VM on the Mac (allowed by the maintainer); the VM's removal deletes it.
- **To check**: whether ttyak renews or rewrites a session while it runs. If it does, the instance's newer copy must
  go back to the master before the claim is released.

## Part 5 — ttyak's docs

- [docs/agents/live-testing.md](../../agents/live-testing.md): Linux live checks run in an instance by default; the
  hidden test workspace on the desktop is kept for what an instance cannot do (named there); every box use and every
  account use goes through its claim.
- [docs/agents/local-template.md](../../agents/local-template.md): fields for the `<tool>` clone and the account
  ledger.
- `AGENTS.local.md` (this machine): rewritten to point at both.
- The other projects on this machine that use the boxes: their git-ignored local files get the same pointer to
  `<tool>`, with the maintainer's word at that step (they are not ttyak's files).

## Prior art (searched 2026-10-27)

- **Claims**: [labgrid](https://github.com/labgrid-project/labgrid) has the semantics wanted (reserve with `--wait`,
  lock, release, a reservation queue) but needs a central coordinator and is built for embedded boards; CI-side
  lockable resources (Jenkins and the like) only serve jobs inside that CI. Neither fits "the lock lives on the box"
  for agents and people; the claim tool stays our own, small, and may borrow labgrid's command semantics.
- **Windows base**: [dockur/windows](https://github.com/dockur/windows) downloads official Windows images by edition
  (Enterprise and evaluation editions among them) and installs unattended. The builder checks whether its download
  and unattended-install pieces (or a similar downloader) can build the evaluation base and remove the manual ISO
  step; the clones themselves stay plain libvirt linked clones.
- **macOS base**: [Tart](https://github.com/openai/tart) plus its
  [Packer builder](https://developer.hashicorp.com/packer/integrations/cirruslabs/tart) (`from_ipsw`: a VM from
  Apple's restore image) replace hand-written base scripts. Orchard (Tart's orchestrator) needs a controller: not
  used. Tart's licence fits: the macOS VMs serve the maintainer's personal projects. (Checked again 2026-10-28: the
  project moved from cirruslabs to OpenAI and is now under the Functional Source License 1.1 with an Apache-2.0
  future licence — any use but offering a competing product is permitted, each version turns Apache-2.0 after two
  years; the earlier Fair Source terms are gone.)
- **Reuse or write** is the builder's decision per piece (the maintainer's word): reuse where a tool fits as it is,
  write our own where fitting a tool would cost more than the piece itself.
- **Hyprland**: its developers say it is [not meant to run headless](https://forum.hypr.land/t/headless-hyprland-in-github-actions/1361)
  and point to NixOS VM tests. If it does not run headless in rootless podman, the Hyprland instance is a small KVM
  Linux VM (a linked clone like Windows, with a virtual display nobody sees) instead of a container; sway (headless
  backend, proven by the spike) and GNOME (mutter's headless mode, used in GTK's CI) stay containers.

**Spike, 2026-10-27 (after the account claims):** Hyprland 0.56 does not start in rootless podman — `CBackend::create()
failed!` headless, with the GPU's render node passed in, and nested in a headless sway — so the Hyprland instance is a
small KVM VM. sway (headless backend) and GNOME's mutter (`--headless --virtual-monitor`) both run in rootless podman
with a terminal window; GNOME's notifications need gnome-shell, not yet tried. The maintainer's decision: container
instances first (sway, then GNOME), then one plan for VM clones (the Hyprland VM and the Windows evaluation base on
the same machinery).

**Spike, 2026-10-28 (VM clones; both answers are yes, with no root and no manual step):**

- **libvirt in user mode** (`qemu:///session`): QEMU runs as the desktop user, the files are the user's own, no
  root-owned pool. UEFI with Secure Boot, a fresh swtpm and a per-clone copy of the NVRAM template all work there.
  passt (libvirt's user networking with port forwards) does not: the distribution's AppArmor profile kills it in
  user mode. QEMU's own user-mode network (SLIRP) replaces it: each VM gets a private network with internet access,
  nothing collides between clones. No display at all: the domain has no `<graphics>` (Linux) or a VNC console on
  127.0.0.1 nobody opens (Windows); `virsh screenshot` captures the screen in both cases, for start-up debugging.
- **Clones**: a qcow2 overlay on a frozen, read-only base, a new domain (new UUID, so a fresh TPM state), a fresh
  NVRAM copy. Two clones started at once did not collide.
- **Hyprland VM**: an Arch cloud image (checksum and signature checked) made into a base by one cloud-init boot
  (95 s). Hyprland 0.56 runs on plain virtio-gpu with Mesa's software renderer (llvmpipe), no extra settings beyond
  a small config (hardware cursor, XWayland and animations off). Agents reach it by ssh over **vsock** (systemd's
  ssh generator in the guest; no network port at all) and drive it with `hyprctl` (open, focus, even type keys with
  `sendshortcut`); mako's list, a PipeWire null sink and `grim` work as in the sway image; a ready file like the
  containers'. Host files are mounted read-only with 9p (set when the clone is defined). Per clone: ready (first
  `hyprctl` answer) ~10 s after start; ~0.5 GiB used in a 2 GiB guest, ~0.9 GiB resident on the host; overlay
  15–30 MB. Base 1.5 GiB. Trap: cloud-init ties the network config to the build VM's MAC — the base build replaces
  it with a match on any ethernet device. Not tried: ttyak raising its own window, ttyak's speed under llvmpipe.
- **Windows evaluation base**: the ISO downloads by script with no form (the Evaluation Center page's link, as
  dockur/windows's MIT-licensed downloader does): Windows 11 Enterprise Evaluation, build 26300, 8.2 GB. Microsoft
  publishes no checksum; size and the image's own metadata are the check. An `autounattend.xml` on a small second
  ISO (with the virtio drivers and the OpenSSH installer) installs it with no interaction in ~6.5 min: a local test
  user with automatic log-on, no Microsoft account screens, no device encryption, indexing, sleep and hibernation
  off, Windows Update off by policy and service, OpenSSH with key auth, the QEMU guest agent, then a shutdown. The
  only key press (the firmware's "boot from CD" prompt) is sent with `virsh send-key`. Checked in a clone: the
  evaluation edition with 90 days, encryption off, the TPM ready, the user logged on, ssh. A clone answers ssh
  9–13 s after start (two at once: 13.5 s); it takes its whole assigned RAM on the host (4 GB at 4 GB; the
  balloon gives it back on request); its overlay grows ~0.4–1 GB. Base 10.4 GiB. Clones share the computer name
  and ssh host key: harmless on one host with private networks, so no sysprep. Trap: the Update service comes back
  once at the base's final shutdown — disabled again at clone start or in a second build boot. The 90 days run from
  the base's install; `slmgr /dlv` over ssh reads what is left. Not tried: notifications and ttyak in a clone.

**VM clones — decided (2026-10-28):**

- The maintainer: **Microsoft Defender off in the Windows base** (faster, quieter clones; tests do not see what
  Defender does to ttyak). Windows 11's tamper protection ignores the old policy switch, so the Windows task starts
  with a short check of how the setup script switches it off for good.
- My calls (the maintainer saw them and did not object):
  - **One tool**: the VMs are instances of the same claim tool — `instance new --desktop hyprland|windows`, the
    same slots, and the same `exec`, `shot`, `notifications`, `windows`, `rm`, `ls`, `reap` commands as the
    containers; only the machinery under them differs (user-mode libvirt, ssh over vsock or a localhost port,
    `virsh screenshot`).
  - **Slots sized per desktop**: a Windows clone holds its whole RAM, so it charges more of the pool than a
    container (about 5 GB with its overlay, Hyprland about 2 GB) instead of one fixed size for every instance.
  - **Overlays in RAM** (tmpfs), charged to the slot: up to ~1 GB per test never reaches the SSD (the anti-goal).
  - **Bases built by script**: the Hyprland base rebuilt on command, the tool warning once it is 30 days old; the
    Windows base downloaded and rebuilt fully by script, the tool warning 14 days before the evaluation ends. Each
    base gets its own ssh key pair, made at build time and kept owner-only beside it.
  - **ttyak's docs**: Windows live checks run in a clone by default; the shared Windows VM is claimed only when a
    test needs that VM itself.

**VM clones — answers for the builder (2026-10-28, from a builder's read; my calls unless marked):**

- **Pool.** Each desktop has a size in the configuration (`instances.sizes_gb`; to start: sway and gnome 1.5,
  hyprland 2.5, windows 5.5 — guest RAM plus QEMU's overhead plus the RAM overlay). The head waiter is granted when
  free memory minus the pending charges minus the reserve covers its size; `max_slots` stays a cap on the count.
  Ticket, slot record and `resume:` line carry the desktop (hence the size). A started VM is still charged what it
  has not taken yet (its size minus its resident memory and overlay bytes), read before the pool lock like the
  stopped-VM reserve; containers stay as they are. FIFO stays strict: a waiting Windows clone holds up smaller
  requests behind it (documented). A base build holds a slot of its desktop's size. Only `qemu:///system` VMs
  count in the reserve; the tool's own VMs live in `qemu:///session` and never do — one URI constant per use.
- **Backends.** `instances.py` is first split behind a backend interface (list, create, remove, exec, ready,
  desktop) with the existing tests unchanged; podman and libvirt are two backends. `new` checks only the chosen
  backend's programs; `ls` and `reap` ask every backend, and one failing does not stop the other.
- **Ownership.** Every VM is a **transient** libvirt domain (`virsh create`: nothing stays defined after a stop or
  a host reboot) carrying the tool's metadata element (pool, claim, desktop, slot, holder, purpose, created, base
  build). `rm`, `ls` and `reap` touch only `qemu:///session` domains with that metadata and this pool; the tool
  never writes to `qemu:///system` (a test asserts it). `reap` also removes per-clone folders without a live claim
  (name `tb-<id>` under the tool's root, checked) and the domain's libvirt logs.
- **Where things live.** Bases: `~/.local/share/<tool>/bases/<desktop>/<build>/` — the base qcow2 (read-only),
  the ssh key pair and the test user's password (made per build, owner-only), `known_hosts`, `meta.json` (built,
  evaluation end, guest uid, sources and checksums); a `current` pointer switched atomically. A rebuild never
  touches a base in use: it goes into a new build folder; an old build is removed only when no clone uses it
  (under a base lock; a build lock stops two builds at once). Per clone: `/dev/shm/<tool>-<uid>/tb-<id>/` (0700)
  — overlay, NVRAM, console log, domain XML. Not `/run/user` (6 GB, the maintainer's session lives there).
  `new` refuses when that tmpfs lacks the overlay cap plus a margin; renewals check the overlay against its cap
  and remove a clone past it. Firmware paths come from the configuration (they differ per distribution).
- **Per-clone values from the claim id, never the slot number**: MAC (a hash), the ssh port (QEMU picks a free
  one, read back), the vsock CID (read from the live domain).
- **ssh.** Our own config only (`-F`, `IdentitiesOnly`, no agent, `BatchMode`); the host key recorded at build
  time and pinned with `HostKeyAlias`; clones share it. Hyprland over vsock through `socat` (a checked
  dependency). cloud-init is disabled at the end of the Hyprland build, which writes the any-ethernet network file
  itself.
- **Ready.** Hyprland: the guest's ready file, at the guest uid recorded in `meta.json`. Windows: a logon task in
  the test user's session writes a ready file once the desktop is up, after checking Update off, Defender off and
  the days left. A per-desktop start timeout (Windows longer than 60 s).
- **Commands.** Hyprland: `exec` over ssh, `windows` from `hyprctl clients -j`, `notifications` from
  `makoctl list -j`, `shot` with grim, `record` from the null sink, as in the containers. Windows: `exec` runs
  through ssh (session 0: files, `ttyak ctl`), `exec --desktop` runs in the logged-on user's session through a
  scheduled task (output and exit code back through files) — the tool's own helper, baked into the base, not
  ttyak's; `shot` with `virsh screenshot`; `windows` and `notifications` through that helper (window list; the
  toast history of ttyak's app id, checked against the code). No VNC listener: no `<graphics>` at all, as on
  Linux. A sound device with a null backend (so ttyak finds an output); `record` on Windows exits 2 for now.
- **Mounts.** Hyprland: `--home` and `--mount` over 9p, mounted by the host over ssh before the session starts
  (the autologin shell waits for a go file; Hyprland's config lives in `/etc`, so a mounted home does not hide
  it; `~/.cache` and `~/.local/state` tmpfs in the guest); the guest user's uid equals the host user's (recorded).
  Windows: `--home`/`--mount` exit 2; new `instance push`/`pull` (scp) copy files in and out — also the account
  session (Part 4). How fast a debug `ttyak.exe` goes over scp is measured; a read-only ISO is the fallback.
- **Windows base build.** The ISO downloaded on command and kept (a reinstall restarts the 90 days without a new
  download); OpenSSH, the virtio drivers and the guest agent pinned by version and checksum. The firmware prompt
  avoided or answered reliably (the builder picks). Defender "off for good" means: after a reboot and in a clone,
  real-time protection and the service are off and its engine does not run; Update stays off across reboots (a
  boot task re-disables it if needed). The build ends with a check pass over ssh (edition, encryption, ssh, the
  guest agent, Defender, Update, log-on, days left) and fails on any miss. The evaluation's end is written at
  build time; the tool warns 14 days ahead and `new` refuses in the last day.
- **Hyprland base build.** The image's signature checked with `gpgv` against a pinned key (no agent prompts); the
  tool warns when the base is 30 days old.
- **Failures.** Undone in reverse (domain if ours, clone folder, logs, slot). No base built: exit 2 with the build
  command; libvirt, QEMU, swtpm, the firmware or socat missing: exit 5.
- **Tests.** Fake `virsh`, `qemu-img` and ssh (as `fake_podman`), the fake refusing any `qemu:///system` write and
  listing foreign domains the tool must ignore; opt-in real-VM tests (`<TOOL>_VM=1`) once bases exist; the base
  builds tested by their generated arguments and files, then built for real.
- **Wording.** The README's "the tool never starts or stops a VM" is about box VMs; the help's "rootless podman"
  becomes "containers or VMs".

**Spike, 2026-10-28 (macOS VM on the Mac; works headless, no click, nothing on the Mac's screen):**

- **Tart** 2.41 (from its signed, notarized release tarball, pinned by checksum: the Homebrew tap's formula is stale
  and refused by Homebrew 7). Every run needs `--no-graphics` (and `CI=true` with VNC, or Tart opens Screen Sharing on
  the host), `--no-audio` (else the guest is wired to the host's real microphone and speakers) and `--no-clipboard`.
  Tart runs as a background process with no window or Dock icon. The lid-closed Mac sleeps between sessions and a
  sleep killed the first install: every VM process runs under `caffeinate -s -i` (an assertion held only while it
  runs; no setting changed).
- **Base**: `tart create --from-ipsw latest` (macOS 26.6.2; a 19.8 GB restore image, cached; install 3 min). Setup
  Assistant is driven headless over VNC by cirruslabs' Packer template's keystrokes, unchanged for 26.6 (account,
  no Apple ID, desktop). Its System Settings route to Remote Login no longer works; keystrokes in the open Terminal
  start sshd instead (persists into clones). Packer's own ssh is blocked by macOS's Local Network privacy (a non-Apple
  binary, refused silently): provisioning runs over Apple's `/usr/bin/ssh` (passwordless sudo, autologin, no sleep or
  screen lock, the key). Terminal's notification permission is granted once in the base by clicks over VNC, so
  clones inherit it. Build from an empty cache ~25 min; base 26 GB on disk.
- **Clones**: `tart clone` (APFS copy-on-write) 0.05 s, 16 KB; ssh ~10 s after start, the desktop session ~10 s;
  4 GB is the image's minimum and the VM holds ~4.1 GB; growth ~0.5 GB per run, all given back by `tart delete`.
  Two at once push the 18 GB Mac into heavy compression; macOS refuses a third. One at a time.
- **Inside**: `open -a Terminal x.command` over ssh runs a program in the logged-on session; a notification posted as
  Terminal (as ttyak's library does) shows as a banner — not when Terminal is the frontmost app (a test brings
  another app forward first); `log show` for usernoted reads what was delivered and presented (its database is
  privacy-protected); the screen is read by VNC screenshots on the Mac (in-VM `screencapture` lacks permission).
  An arm64 binary copied in by scp runs with Gatekeeper on; a read-only `--dir` share also works.
- **From the desktop**: the VM sits behind Tart's NAT on the Mac, not reachable from the desktop; everything goes
  through the Mac (piping through the claim tool moved 100 MB in 2.2 s). Helpers on the Mac that connect to the VM
  must be Apple binaries (`/usr/bin/ssh`, `nc`) because of Local Network privacy.

**macOS VM — my calls (2026-10-28, for the maintainer's approval):**

- The Mac holds its own slot pool (one slot, ~4.5 GB) and its own bases; `instance … --desktop macos` on the desktop
  runs the claim tool on the Mac over ssh, as box claims do. Using the VM takes the Mac's VM slot, not the Mac box
  claim (as decided above).
- ttyak for the VM is built on the Mac (in the agents' work folder, not in the VM) and shared read-only with `--dir`;
  the session goes in by `push`.
- The base build: Packer with only the keystrokes, then our own provisioning over `/usr/bin/ssh`; the notification
  grant by Packer's text-recognition click, coordinates as the fallback; Tart pinned by version and checksum.
- A running VM keeps the Mac awake (`caffeinate`), only while it runs.
- The maintainer approved these (2026-10-28) and set the Mac's reserve: **6 GB** always free for them.

**macOS VM — answers for the builder (2026-10-28, from a builder's read; my calls unless marked):**

- **Routing.** The box list says which desktops a box hosts (`"desktops": ["macos"]` on the Mac). A macOS ticket and
  claim id start with the box's name (`mac-…`), and every instance command routes on that prefix: the desktop keeps
  no state. `new` refuses a local ticket that starts with a box's name.
- **Relay.** The desktop runs the Mac's tool (the box's `remote`) over `ssh -T` with a relay flag and a protocol
  version (an outdated Mac clone refuses with "git pull on the Mac"); the Mac sends `resume:`/`rm:` lines as data and
  the desktop renders them for its own shell. Ctrl-C or a killed desktop agent closes ssh; on the Mac a thread sees
  stdin end and runs the existing Ctrl-C cleanup (leave the queue, remove the VM, release the slot); `ServerAlive`
  covers a silent network. Exit 255 on the desktop always means the Mac is unreachable (reported as 5); the Mac maps
  its own inner 255s to 5.
- **Files.** push/pull checks run on the desktop; files travel as a tar stream desktop → Mac → VM (pull extracts by
  hand: regular files and folders only, no links, no `..`, no absolute names); `shot` streams the PNG back and the
  desktop writes it 0600.
- **Box and slot.** A withdrawn Mac refuses new macOS slots (exit 4; running VMs finish); an unreachable or asleep
  Mac is exit 5. `ls` on the desktop adds the Mac's rows or an "unreachable" note; `reap` is local to each host.
- **Pool on the Mac.** Per-host settings in the box list (`instances.hosts.<self>`): reserve 6 GB (the maintainer),
  `max_slots` 1, `macos` 4.5 GB. Memory on macOS: available = `hw.memsize` × macOS's own free percentage
  (`kern.memorystatus_level`, what `memory_pressure` shows), and no grant while macOS reports memory pressure; the
  Mac counts no libvirt VMs. (The maintainer, 2026-10-28: a stricter count from `vm_stat` — compressible app memory
  and the compressor as used — left a VM never fitting beside a browser while macOS reported 82% free.) A started VM is charged nothing more (resident at ~4.1 GB once ready).
- **Tart.** The release bundle pinned by version and SHA-256, fetched with Apple's `/usr/bin/curl`, kept whole (its
  entitlements) under the tool's own folder; `TART_HOME` the tool's own (never `~/.tart`); auto-prune off (it would
  drop the restore image). Every run: `--no-graphics --no-audio --no-clipboard`, VNC on 127.0.0.1 with `CI=true`.
  Refused on battery (exit 5): `caffeinate -s` holds only on AC.
- **Keeper.** A detached keeper process per VM (its own session) runs `caffeinate -s -i tart run …`, records the PIDs
  and the VNC address (0600), and every minute stops and deletes the VM once its slot claim is not live — no launchd
  job in the maintainer's account. `reap` also runs at every Mac-side `new` and `ls`.
- **Ownership.** Everything in the tool's `TART_HOME` is the tool's; clones `tb-<id>`, bases `base-macos-<build>`; a
  record per clone (pool, claim, slot, holder, purpose, created, build); before every ssh the record, the running VM
  and the keeper are checked. Apple's two-VM limit counts any VM of the maintainer's: mapped to a clear failure.
- **Inside.** `exec` through Apple's `/usr/bin/ssh` with the base's key and pinned host key (the tool itself never
  opens a socket to the VM: Local Network privacy). `exec --desktop` writes a `.command` in a run folder and starts
  it with `open -g -a Terminal` (in the background, so banners show), output and exit code through files, as on
  Windows. `shot` by a small stdlib VNC client on the Mac (VNC auth with a pure-Python DES, known-answer tests), not
  in-guest capture (its permission expires monthly). `windows` from `lsappinfo` (apps and the front one; no titles,
  documented). `notifications` from `log show` for usernoted (delivered/presented and the app; title and body if the
  log shows them). `record` exits 2 (no audio device with `--no-audio`; sound checks are out of scope on macOS).
  Ready: a LaunchAgent in the base writes a ready file after log-on (sshd, the session, the Focus state); 90 s.
- **Base build.** Runs on the Mac only, under the Mac box claim and a slot of the macOS size, started through the box
  tool (not relayed). Packer pinned by checksum, its plugin pinned, checkpoint off, its folders the tool's own,
  communicator none, the keystroke template vendored from cirruslabs at a pinned commit (licence checked); the
  Terminal notification grant by Packer's text-recognition click, coordinates as fallback. A user `tb` with a random
  password per build (0600), automatic log-on, passwordless sudo; Software Update, Spotlight indexing, sleep, screen
  lock and assistant prompts off. A check pass on a clone made as `new` makes it, before publish. Restore image
  cached; rebuilt from a fresh download when Apple no longer signs the cached one. Only the current build is kept;
  a build refuses below a free-disk floor.
- **ttyak in the VM.** Built on the Mac under a Mac box claim; `--mount SRC:NAME` snapshots SRC with an APFS clone
  (`cp -c`, free) into the clone's record folder and shares it read-only at `/Volumes/My Shared Files/NAME`. The
  session is pushed to ttyak's folder in the VM (`~/Library/Application Support/ttyak/<profile>`); the VM is the
  fake home. Whether ttyak touches the Keychain is checked (a prompt would hang unseen).
- **SSD.** A clone writes ~0.5 GB per run to the Mac's SSD (APFS clones cannot live in RAM), removed with it; the
  anti-goal is met by size, not by RAM.
- **Tests.** Fakes on the desktop for tart, caffeinate, `vm_stat`/`sysctl`/`pmset`, and the Mac hop (a local tool
  run as the Mac with its own folders); the macOS modules import on Linux. Real tests opt-in on the Mac under a box
  claim, using the real one-slot pool.
- **Generalised from the VM code:** bases' `build|ls|gc` per backend; the reap timer is the desktop's (systemd).

## Order (my call)

1. `<tool>` repository and box claims — the collisions hurt today.
2. ttyak account ledger and claims.
3. Linux instances (containers: sway, then GNOME; Hyprland moved to the VMs after the spike), then ttyak's
   live-testing docs switch to them.
4. VM clones: the Hyprland VM and the Windows evaluation base.
5. macOS base and VM on the Mac.

Each step is usable on its own. The multi-agent workflow ([#29](https://github.com/dmytrobsp/ttyak/issues/29)) builds
on 1–3.
