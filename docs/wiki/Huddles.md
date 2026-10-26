A huddle is Slack's call. In ttyak you join and start huddles, talk and listen, see other people's cameras and screen
shares, and send your own camera and screen. The keys are the `␣h` group (see [Keys](Keys#the-other-groups)); what each
part needs installed (`ffmpeg`, GStreamer) is on [Requirements](Requirements).

## Joining and leaving

- `␣hh` joins (or starts) the huddle of the conversation you are in; in a huddle it leaves. In the
  [Huddles view](The-screen#the-huddles-view) it joins the live huddle selected there; while an invitation rings
  (below), it joins that one from any conversation.
- `␣hT` on a message starts (or joins) a huddle in that message's thread; `␣ht` opens the huddle's thread. On a
  profile card `Ctrl+Space h p` starts a huddle with that person.
- You join **muted**. The microphone is opened only while you are unmuted: `␣hm` mutes and unmutes.
- Leaving, or quitting ttyak in a huddle, also tells Slack (`rooms.leave`, as its own client does), so the huddle's
  list drops you at once.

While you are in a huddle, others see you "In a huddle" (🎧 when you have no status emoji), as with Slack's own app:
Slack sets it when you join and clears it when you leave, and ttyak renews it every ~4 minutes, as Slack's app does —
only while your Slack preference for it (`huddles_set_status`) is on, and never touching your status.

## The call bar

While you are in a huddle, a bar at the bottom shows it: 🎧 and where the huddle is, "connecting…" or "reconnecting…"
while it joins, the picture and name of whoever is talking with their level ("talked last" once they stop), "just you"
or "N here", your microphone (🎙 live with your level, or 🔇 muted), "📹 camera on" or "camera off", and a screen share
("🖥 sharing your screen", or "🖥 <name> is sharing"). When the speaker cannot be opened it says "no sound: …". In a
narrow window the bar takes more rows rather than cutting a piece in half (up to four rows; what does not fit then is
left out). It shows no keys: after `␣h` the which-key panel lists them. A click on the bar opens the huddle view.

## The huddle view

`␣hv` (or a click on the call bar) shows the huddle view in place of the messages: a tile per participant with their
picture or video — the border lights up while they speak, 🔇 in the title when muted. The tiles shrink as people join
and scroll (mouse wheel, "▲/▼ N more") when they no longer fit; a screen share takes the stage with everyone else in a
strip. A line under the tiles shows the audio settings (volume, microphone gain, threshold, noise suppression, echo
cancellation).

↑ ↓ ← → (or `h` `j` `k` `l`) select a tile, Enter (or `f`, or a second click) maximizes it, Esc shows all tiles again
and then leaves the tiles. `␣hv` again, or opening a conversation, brings the messages back.

## Invitations

Someone starting a huddle in a DM or group DM with you sends a desktop notification
(see [Notifications and attention](Notifications-and-attention)) and, while you are not in a huddle yourself, the
invitation rings: a soft trill every few seconds, with "📞 X invites you to a huddle · ␣hh join · ␣hd decline" in the
status line. It rings until you join (`␣hh`, from any conversation), decline (`␣hd`), the invitation is withdrawn
(the caller left, or the huddle ended), or it has rung for the set time (30 s by default); then it is declined for you
if "Then decline it on Slack" is on, else the status line says "missed X's huddle".

While you are in Do Not Disturb (snoozed or in your schedule), Slack sends no invitation at all, so nothing rings — as
in Slack's own app (tried). The system's do-not-disturb keeps the ring quiet when ttyak follows it and can ask (not on
macOS): see [Do-not-disturb](Notifications-and-attention#do-not-disturb).

Clicking the huddle's desktop notification opens its conversation in ttyak, for 30 minutes after it was shown, and
brings the window forward where it can (Linux desktops with an XDG notification server; macOS and Windows differ: see
[A click brings the window forward](Notifications-and-attention#a-click-brings-the-window-forward)).

## Sounds (F6 → Huddles)

Each can be turned off or changed; they play at the notification sound volume (F6 → Notifications) on the speaker
chosen in F6:

- Sound when you join (`rise`: a soft rising arpeggio), when someone joins (`pop`) and when someone leaves (`fall`: a
  falling pair)
- An invitation rings with (`ring`), Stop ringing after (15 to 120 s, 30 by default), Then decline it on Slack (off by
  default)
- Alone in a huddle, loop (`lullaby`, a quiet melody, at 40 % of the volume) and Start it after (30 s to 15 minutes,
  300 s by default): when you have been the only one in a huddle that long, it loops until someone joins or you leave

## When the connection drops

If a huddle's connection is lost (a network change, say), ttyak joins again by itself, as Slack's app does: the status
line says "huddle connection lost — joining again…" and the call bar "reconnecting…" (with no microphone level
meanwhile), then "back in the huddle", usually a few seconds later (tried). Mute and camera stay as they were, also when
changed meanwhile.

Once you were connected, any trouble with the connection joins again: the media failing or staying disconnected for
4 s, a signaling error from the server, a broken signaling socket. The call ends when you leave (also while it is
joining again), when the server closes the signaling connection (the huddle ended, or you were removed), when the very
first join fails, after three tries in a row that do not stay up for 30 s (0, 2 and 4 s apart), or after more than ten
reconnects within an hour. In those last two cases ttyak tells Slack it left, as when you leave, and the status line
says "huddle connection lost — left the huddle".

## Audio (F6 → Audio & video)

- Microphone and Speaker: the system's default until you choose one. `TTYAK_AUDIO_INPUT` / `TTYAK_AUDIO_OUTPUT` (part
  of a device id or name from `ttyak ctl audio-devices`, any case) win over them. A device that is named but missing
  is an error; there is no silent fallback.
- Huddle volume (0 to 200 %) and Microphone gain (25 to 400 %).
- Speaking threshold (off by default): quieter microphone input is sent as silence, held open 300 ms after you stop so
  words are not cut.
- Noise suppression (RNNoise via `nnnoiseless`, off by default).
- Echo cancellation (on by default: WebRTC's AEC3 via the pure-Rust `sonora`, so others do not hear themselves from
  your speakers; it drops the microphone above 16 kHz, which speech does not need).

`+` / `-` (or Enter) step the levels, and a huddle you are in follows at once. The notification sound volume, which the
huddle sounds use, is in F6 → Notifications.

## Video: cameras and screen shares

Other people's cameras play in their tiles in the huddle view (up to nine at a time, fewer if the server says so); a
screen share gets its own "🖥 <name>'s screen" tile ("🖥 <name> is sharing" in the call bar). Video is H.264 (OpenH264,
built from source — note that H.264 is patent-encumbered when you distribute binaries).

**Your camera**: choose it in F6 → Audio & video → Camera (until you do, it can never switch on), then `␣hc` in a
huddle switches it on and off; you see yourself in your tile. Capture goes through the `ffmpeg` tool: V4L2 on Linux,
AVFoundation on macOS, DirectShow on Windows. On macOS and Windows the cameras are listed with ffmpeg's
`-list_devices`. On Windows that list has run, on a machine without a camera (it came back empty); capture on macOS and
Windows, and the macOS list, are written but have not been run yet.

**Your screen**: `␣hs` in a huddle starts sharing, `␣hs` again stops. Others see it as your screen share; you see it in
a "🖥 Your screen" tile.

- In a Wayland session the desktop asks what to share (a screen or a window), through the screen-cast portal, read
  with GStreamer's `pipewiresrc` — `gst-launch-1.0` and the GStreamer PipeWire plugin must be installed.
- Elsewhere there is no picker and `␣hs` shares the **whole** screen through `ffmpeg`: `x11grab` in an X11 session, the
  main screen via AVFoundation on macOS (the terminal needs the Screen Recording permission), the desktop via `gdigrab`
  on Windows.

## Where it has been tried

- Linux: huddles with a second account, the reconnect, invitations, screen sharing through the Wayland portal (a whole
  monitor and a single window) and through X11 (against a virtual Xvfb display), a camera through a virtual V4L2
  device. Not tried: a real webcam.
- macOS and Windows: joining a huddle, sending a test tone and a test camera, and receiving other people's sound,
  camera and screen share (`ctl huddle-media`, from an SSH session). Not tried: the speaker, the microphone, a real
  camera, and sharing the screen (on Windows only from an SSH session, where `gdigrab` cannot capture: no desktop).

## Tests

`ttyak ctl huddle-media <channel> [seconds]` joins a huddle without the TUI: `--tone HZ` sends a test tone, `--mic` the
default microphone, `--record FILE` saves the sound that arrives (48 kHz mono s16le), `--video-test` joins with a
synthetic camera (a moving test pattern), `--share-test` also shares it as a screen, `--camera NAME` sends a real camera
(`ctl cameras` lists them), `--share-screen` shares your screen and `--record-video DIR` saves others' video (a PNG per
person every 2 seconds). See [Scripting](Scripting).

## How it works

Under the hood a huddle is an Amazon Chime SDK meeting: `rooms.join` hands out the meeting credentials, `crates/huddle`
speaks Chime's signaling protocol and runs WebRTC audio and video (webrtc-rs, Opus, H.264). How this was worked out is
in [`docs/protocol/huddles.md`](https://github.com/dmytrobsp/ttyak/blob/master/docs/protocol/huddles.md). ttyak carries
a patched copy of webrtc-rs' TURN client that keeps a huddle's audio from dropping about ten minutes in (see
[Project layout](Project-layout)).

## If it doesn't work

- [Huddle audio: no sound, or the wrong microphone or speaker](Troubleshooting#huddle-audio-no-sound-or-the-wrong-microphone-or-speaker)
- [Video, the camera or screen sharing doesn't work](Troubleshooting#video-the-camera-or-screen-sharing-doesnt-work)
- [A huddle invitation doesn't ring](Troubleshooting#a-huddle-invitation-doesnt-ring)
- [A click on a notification doesn't bring the window forward](Troubleshooting#a-click-on-a-notification-doesnt-bring-the-window-forward)
- everything else: [Troubleshooting](Troubleshooting)
