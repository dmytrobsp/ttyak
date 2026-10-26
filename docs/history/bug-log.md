# Bug log

- The status bar's "N unread" counted conversations the sidebar does not show as unread — muted ones (dimmed, as in
  Slack), archived channels, DMs with deactivated people — so it could say "4 unread" with nothing to find. Reported by
  the user from their work workspace (P1); reproduced with a muted test channel; the count now uses the sidebar's own
  rule (`the_unread_count_is_what_the_sidebar_shows`).

- The composer's cursor was cells away from the text once a word wrapped: the text was drawn word-wrapped by ratatui
  while the cursor was counted as if rows broke at any character. Reported by the user from real use (M1); the composer
  now wraps the text itself and the drawing and the cursor share those rows (`the_cursor_follows_wrapped_words`).

- A message taller than the message pane flickered between its start and its end when selected: the selection was kept
  on screen at every draw, by its last line when its end was below the view and by its first when its start was above —
  a message taller than the view always had one of the two, so each redraw (each pointer move) flipped it. Found by the
  user on their work workspace (L1). The selection is now brought into view once, when it changes, a tall one from its
  start (regression test `a_message_taller_than_the_view_stays_put_when_selected`).

- Attachments without a title showed their `fallback` as a bold title above their text — `fallback` is a plain-text
  copy, so the text came twice, the first time raw (backticks, `<url|label>`). Found in I6 with Slackbot's answer to
  `/remind help`; `fallback` now shows only when there is no title and no text (regression test
  `attachments_show_their_fallback_only_when_there_is_nothing_else`).

- A picture first drawn while a dialog covered it never showed (an empty box until restart): ratatui-image sends a
  picture to kitty once, in front of its first placeholder cell, and marks it sent; the dialog's `Clear` (and its text)
  overwrote that cell in the same frame. Found in H4h by turning two-row pictures on in F6 (the new pictures load under
  the dialog). `clear_popup` now holds that data and the frame puts it back in front of whatever covers the cell (a
  virtual placement shows nothing there), with the cell's own width forced for ratatui's diff (regression test
  `pictures_first_drawn_under_a_popup_still_reach_the_terminal`).

- A window squeezed to a few cells crashed klack ("min > max" in `f64::clamp`): image thumbnails were sized with a
  maximum of 0 columns. Found in H3 by shrinking the test window step by step with popups open; a pane with no room now
  gets no thumbnail (regression test `no_room_no_thumbnail`).

- Up on the oldest loaded message never loaded older history when the lines above it (its day separator) were off
  screen: `maybe_load_older` needed the very top line visible, and keeping the selection in view stops at its own lines.
  Found in the F3f regression test (the previous binary did the same); Up there now loads older messages directly
  (`load_history` guards against double loads and the end of history).

- An edit started in one conversation and finished after switching to another was looked up in the new conversation and
  silently lost (one composer for all conversations; `save_edit` uses the current one). Found by reading the code while
  adding drafts (not seen live); drafts now keep the edit with its conversation. The single composer also carried typed
  text into the next conversation.

- Muted channels were never recognised: boot's `prefs.muted_channels` is null on current workspaces; mutes live in
  `all_notifications_prefs.channels[*].muted`. Found while reverse engineering the notification prefs; now read from
  there (and live from `pref_change`). Test `notification_prefs_from_slack`.

- B's sidebar listed a DM as "USLACK": Slack's system user (`USLACK`, "Slack", trial notices) is not returned by
  `users.list`. Seen while testing workspace switching; DM partners missing from the user list are now looked up with
  `users.info` after boot. Test `dm_partners_missing_from_the_user_list`; live, B's sidebar shows "Slack".

- Scheduled quiet hours were assumed to repeat every day, so with a weekday-only schedule (17:00–09:00) the app thought
  DND was on on a Sunday evening and would have kept notifications quiet; seen as a stray 🔕 in the new status bar.
  `dnd.info` then reports the next window (Mon 17:00 → Tue 09:00). Now only the window ending at `next_dnd_end_ts`
  counts (its length being the schedule's daily span), and `dnd.info` is asked again once it has passed. The
  `dnd_windows` test has the real values.

- Day separators and the thread "N replies" rule were attributed to the next message, so selecting it highlighted the
  rule too. Found by screenshot after a search jump; tests now assert they belong to no message.

- The `WideSafe` filter (previous fix) measured the raw cell symbol, so a kitty image cell carrying the image
  transmission escape (thousands of characters, forced width 1) looked thousands of columns wide and the rest of its row
  was dropped: the viewer image lost its top row. Found by screenshot; now uses `Cell::cell_width()`, which honours
  forced widths. Test `forced_width_cells_cover_nothing`.

- VS16 emoji (❤️) shifted the rest of their row by one column (`[❤ 1]`) and left stale characters after scrolling or
  switching conversation. Reproduced in isolation with a two-frame ratatui program in kitty. Cause: ratatui-core 0.1.2
  re-emits the emoji's trailing cell right after it; the crossterm backend sees an adjacent cell and skips the cursor
  move, but the terminal had already advanced two columns. Fixed with a backend wrapper (`terminal::WideSafe`) that
  drops writes into columns covered by the wide character just written. Test `covered_columns_are_dropped`; the same
  live scenario is clean now.

- Notifications never fired: `dnd.info` reports the *next start* and *next end* separately (today 00:00 and tomorrow
  09:00 for a daily 00:00–09:00 schedule), and reading them as one span made DND look active for 33 h. Found with a
  debug log of each notification decision; now read as a daily schedule. Test `dnd_windows` uses the real values.

- `message_replied` carries the parent *without* reactions/files, and replacing the parent wiped its reactions (👍
  vanished after any reply). Confirmed by capturing A's RTM events with `ctl listen`; now only the thread fields are
  merged. `message_changed` likewise keeps reactions, reply counts and files when the event omits them (a missing
  `reply_count` reset it to 0, caught by the new test). Test `message_replied_keeps_reactions`.

- Live thread replies were counted twice (`💬 3 replies` for 2): the reply event bumped `reply_count` and
  `message_replied` also carried the server count. Found by screenshot; a reply is now counted only if newer than the
  parent's `latest_reply`. Test `reply_counted_once_with_message_replied`.

- " (edited)" was appended after wrapping and got cut off in narrow panes. Found by screenshot in the thread pane; now
  wraps to its own line. Test `edited_mark_wraps_instead_of_overflowing`.

- Duplicate day separator after system messages (`channel_join`). Found by screenshot; fixed in 962ca69 and guarded by
  the test `one_day_separator_around_system_messages`.

- Status-bar unread counts were computed only at boot. Found by screenshot; now computed per frame (3acf67d).
