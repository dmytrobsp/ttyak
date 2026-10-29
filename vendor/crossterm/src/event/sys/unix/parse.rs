use std::{collections::VecDeque, io};

use crate::event::{
    Event, KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers, KeyboardEnhancementFlags,
    MediaKeyCode, ModifierKeyCode, MouseButton, MouseEvent, MouseEventKind,
};

use super::super::super::InternalEvent;

// Event parsing
//
// This code (& previous one) are kind of ugly. We have to think about this,
// because it's really not maintainable, no tests, etc.
//
// Every fn returns Result<Option<InputEvent>>
//
// Ok(None) -> wait for more bytes
// Err(_) -> failed to parse event, clear the buffer
// Ok(Some(event)) -> we have event, clear the buffer
//

fn could_not_parse_event_error() -> io::Error {
    io::Error::new(io::ErrorKind::Other, "Could not parse an event.")
}

// ttyak: the longest OSC reply read (bytes, `ESC ]` included). A real reply from kitty (`ESC ] 99 ; i=<id> ; …`) is a
// few dozen bytes; past this the bytes are given back as ordinary input (see `advance_bytes`).
const OSC_MAX_LEN: usize = 256;

// ttyak: the start every OSC reply ttyak reads has after `ESC ]`: kitty's notification report.
const OSC_START: &[u8] = b"99;";

// ttyak: whether `next`, coming after `buffer` (which starts with `ESC ]`), cannot belong to an OSC reply: not the
// start `99;`, then not a printable ASCII byte without space, BEL or ESC; an ESC must be followed by `\`; and not past
// `OSC_MAX_LEN`. Then the OSC is given up (see `advance_bytes`). Also covers `ESC ]` and a byte that is no `9`: Alt+].
fn osc_broken_by(buffer: &[u8], next: u8) -> bool {
    if buffer.len() < 2 || buffer[0] != b'\x1B' || buffer[1] != b']' {
        return false;
    }
    let body = &buffer[2..];
    if body.len() < OSC_START.len() {
        next != OSC_START[body.len()]
    } else if buffer.len() >= OSC_MAX_LEN {
        true
    } else if body.last() == Some(&b'\x1B') {
        next != b'\\'
    } else {
        !(next == b'\x07' || next == b'\x1B' || (0x21..=0x7E).contains(&next))
    }
}

// ttyak: the reader's loop over `input`, the way both readers (`source/unix/mio.rs`, `tty.rs`) used to have it, plus:
// when an OSC cannot go on (`osc_broken_by`), its `ESC ]` is Alt+] and everything after it, `next` included, is fed
// again as ordinary input. Nothing is lost: a stray `ESC ] 9 9 ; h` is Alt+], `9`, `9`, `;`, `h`, and an `ESC` inside it
// starts the sequence that follows (a mouse move after a stray OSC stays a mouse move).
pub(crate) fn advance_bytes(
    buffer: &mut Vec<u8>,
    events: &mut VecDeque<InternalEvent>,
    input: &[u8],
    more: bool,
) {
    let mut pending: VecDeque<u8> = input.iter().copied().collect();
    while let Some(byte) = pending.pop_front() {
        // `more`: further bytes of this read are waiting, or the caller says so.
        let more = !pending.is_empty() || more;

        if osc_broken_by(buffer, byte) {
            pending.push_front(byte);
            give_up_osc(buffer, events, &mut pending);
            continue;
        }

        buffer.push(byte);

        match parse_event(buffer, more) {
            Ok(Some(ie)) => {
                events.push_back(ie);
                buffer.clear();
            }
            Ok(None) => {
                // Event can't be parsed, because we don't have enough bytes for
                // the current sequence. Keep the buffer and process next bytes.
                // ttyak: except an OSC with nothing more to read right now: a reply comes in one piece, so
                // this one is a stray and gives its bytes back.
                if !more && buffer.starts_with(b"\x1B]") {
                    give_up_osc(buffer, events, &mut pending);
                }
            }
            Err(_) => {
                // Event can't be parsed (not enough parameters, parameter is not a number, ...).
                // Clear the buffer and continue with another sequence.
                buffer.clear();
            }
        }
    }
}

// ttyak: the `ESC ]` of the OSC in `buffer` is Alt+]; the rest of `buffer` goes back in front of `pending`.
fn give_up_osc(buffer: &mut Vec<u8>, events: &mut VecDeque<InternalEvent>, pending: &mut VecDeque<u8>) {
    events.push_back(InternalEvent::Event(Event::Key(KeyEvent::new(
        KeyCode::Char(']'),
        KeyModifiers::ALT,
    ))));
    for held in buffer.drain(..).skip(2).rev() {
        pending.push_front(held);
    }
}

// ttyak: `ESC ] 99 ; …` up to BEL or `ESC \`, as `Event::Osc`. `None` while the end has not come (`advance_bytes`
// gives up one longer than `OSC_MAX_LEN` before it gets here).
fn parse_osc(buffer: &[u8]) -> io::Result<Option<InternalEvent>> {
    let osc = |payload: &[u8]| {
        Ok(Some(InternalEvent::Event(Event::Osc(
            String::from_utf8_lossy(payload).into_owned(),
        ))))
    };
    match buffer {
        [.., b'\x07'] => osc(&buffer[2..buffer.len() - 1]),
        [.., b'\x1B', b'\\'] => osc(&buffer[2..buffer.len() - 2]),
        _ => Ok(None),
    }
}

pub(crate) fn parse_event(
    buffer: &[u8],
    input_available: bool,
) -> io::Result<Option<InternalEvent>> {
    if buffer.is_empty() {
        return Ok(None);
    }

    match buffer[0] {
        b'\x1B' => {
            if buffer.len() == 1 {
                if input_available {
                    // Possible Esc sequence
                    Ok(None)
                } else {
                    Ok(Some(InternalEvent::Event(Event::Key(KeyCode::Esc.into()))))
                }
            } else {
                match buffer[1] {
                    b'O' => {
                        if buffer.len() == 2 {
                            Ok(None)
                        } else {
                            match buffer[2] {
                                b'D' => {
                                    Ok(Some(InternalEvent::Event(Event::Key(KeyCode::Left.into()))))
                                }
                                b'C' => Ok(Some(InternalEvent::Event(Event::Key(
                                    KeyCode::Right.into(),
                                )))),
                                b'A' => {
                                    Ok(Some(InternalEvent::Event(Event::Key(KeyCode::Up.into()))))
                                }
                                b'B' => {
                                    Ok(Some(InternalEvent::Event(Event::Key(KeyCode::Down.into()))))
                                }
                                b'H' => {
                                    Ok(Some(InternalEvent::Event(Event::Key(KeyCode::Home.into()))))
                                }
                                b'F' => {
                                    Ok(Some(InternalEvent::Event(Event::Key(KeyCode::End.into()))))
                                }
                                // F1-F4
                                val @ b'P'..=b'S' => Ok(Some(InternalEvent::Event(Event::Key(
                                    KeyCode::F(1 + val - b'P').into(),
                                )))),
                                _ => Err(could_not_parse_event_error()),
                            }
                        }
                    }
                    b'[' => parse_csi(buffer),
                    // ttyak: `ESC ] 9` starts an OSC reply (`advance_bytes` gives up one that is not `ESC ] 99 ; …`);
                    // `ESC ]` alone waits for the next byte while more input is pending, else it is Alt+] as before.
                    b']' if buffer.len() == 2 && input_available => Ok(None),
                    b']' if buffer.len() > 2 && buffer[2] == b'9' => parse_osc(buffer),
                    b'\x1B' => Ok(Some(InternalEvent::Event(Event::Key(KeyCode::Esc.into())))),
                    _ => parse_event(&buffer[1..], input_available).map(|event_option| {
                        event_option.map(|event| {
                            if let InternalEvent::Event(Event::Key(key_event)) = event {
                                let mut alt_key_event = key_event;
                                alt_key_event.modifiers |= KeyModifiers::ALT;
                                InternalEvent::Event(Event::Key(alt_key_event))
                            } else {
                                event
                            }
                        })
                    }),
                }
            }
        }
        b'\r' => Ok(Some(InternalEvent::Event(Event::Key(
            KeyCode::Enter.into(),
        )))),
        // Issue #371: \n = 0xA, which is also the keycode for Ctrl+J. The only reason we get
        // newlines as input is because the terminal converts \r into \n for us. When we
        // enter raw mode, we disable that, so \n no longer has any meaning - it's better to
        // use Ctrl+J. Waiting to handle it here means it gets picked up later
        b'\n' if !crate::terminal::sys::is_raw_mode_enabled() => Ok(Some(InternalEvent::Event(
            Event::Key(KeyCode::Enter.into()),
        ))),
        b'\t' => Ok(Some(InternalEvent::Event(Event::Key(KeyCode::Tab.into())))),
        b'\x7F' => Ok(Some(InternalEvent::Event(Event::Key(
            KeyCode::Backspace.into(),
        )))),
        c @ b'\x01'..=b'\x1A' => Ok(Some(InternalEvent::Event(Event::Key(KeyEvent::new(
            KeyCode::Char((c - 0x1 + b'a') as char),
            KeyModifiers::CONTROL,
        ))))),
        c @ b'\x1C'..=b'\x1F' => Ok(Some(InternalEvent::Event(Event::Key(KeyEvent::new(
            KeyCode::Char((c - 0x1C + b'4') as char),
            KeyModifiers::CONTROL,
        ))))),
        b'\0' => Ok(Some(InternalEvent::Event(Event::Key(KeyEvent::new(
            KeyCode::Char(' '),
            KeyModifiers::CONTROL,
        ))))),
        _ => parse_utf8_char(buffer).map(|maybe_char| {
            maybe_char
                .map(KeyCode::Char)
                .map(char_code_to_event)
                .map(Event::Key)
                .map(InternalEvent::Event)
        }),
    }
}

// converts KeyCode to KeyEvent (adds shift modifier in case of uppercase characters)
fn char_code_to_event(code: KeyCode) -> KeyEvent {
    let modifiers = match code {
        KeyCode::Char(c) if c.is_uppercase() => KeyModifiers::SHIFT,
        _ => KeyModifiers::empty(),
    };
    KeyEvent::new(code, modifiers)
}

pub(crate) fn parse_csi(buffer: &[u8]) -> io::Result<Option<InternalEvent>> {
    assert!(buffer.starts_with(b"\x1B[")); // ESC [

    if buffer.len() == 2 {
        return Ok(None);
    }

    let input_event = match buffer[2] {
        b'[' => {
            if buffer.len() == 3 {
                None
            } else {
                match buffer[3] {
                    // NOTE (@imdaveho): cannot find when this occurs;
                    // having another '[' after ESC[ not a likely scenario
                    val @ b'A'..=b'E' => Some(Event::Key(KeyCode::F(1 + val - b'A').into())),
                    _ => return Err(could_not_parse_event_error()),
                }
            }
        }
        b'D' => Some(Event::Key(KeyCode::Left.into())),
        b'C' => Some(Event::Key(KeyCode::Right.into())),
        b'A' => Some(Event::Key(KeyCode::Up.into())),
        b'B' => Some(Event::Key(KeyCode::Down.into())),
        b'H' => Some(Event::Key(KeyCode::Home.into())),
        b'F' => Some(Event::Key(KeyCode::End.into())),
        b'Z' => Some(Event::Key(KeyEvent::new_with_kind(
            KeyCode::BackTab,
            KeyModifiers::SHIFT,
            KeyEventKind::Press,
        ))),
        b'M' => return parse_csi_normal_mouse(buffer),
        b'<' => return parse_csi_sgr_mouse(buffer),
        b'I' => Some(Event::FocusGained),
        b'O' => Some(Event::FocusLost),
        b';' => return parse_csi_modifier_key_code(buffer),
        // P, Q, and S for compatibility with Kitty keyboard protocol,
        // as the 1 in 'CSI 1 P' etc. must be omitted if there are no
        // modifiers pressed:
        // https://sw.kovidgoyal.net/kitty/keyboard-protocol/#legacy-functional-keys
        b'P' => Some(Event::Key(KeyCode::F(1).into())),
        b'Q' => Some(Event::Key(KeyCode::F(2).into())),
        b'S' => Some(Event::Key(KeyCode::F(4).into())),
        b'?' => match buffer[buffer.len() - 1] {
            b'u' => return parse_csi_keyboard_enhancement_flags(buffer),
            b'c' => return parse_csi_primary_device_attributes(buffer),
            _ => None,
        },
        b'0'..=b'9' => {
            // Numbered escape code.
            if buffer.len() == 3 {
                None
            } else {
                // The final byte of a CSI sequence can be in the range 64-126, so
                // let's keep reading anything else.
                let last_byte = buffer[buffer.len() - 1];
                if !(64..=126).contains(&last_byte) {
                    None
                } else {
                    #[cfg(feature = "bracketed-paste")]
                    if buffer.starts_with(b"\x1B[200~") {
                        return parse_csi_bracketed_paste(buffer);
                    }
                    match last_byte {
                        b'M' => return parse_csi_rxvt_mouse(buffer),
                        b'~' => return parse_csi_special_key_code(buffer),
                        b'u' => return parse_csi_u_encoded_key_code(buffer),
                        b'R' => return parse_csi_cursor_position(buffer),
                        _ => return parse_csi_modifier_key_code(buffer),
                    }
                }
            }
        }
        _ => return Err(could_not_parse_event_error()),
    };

    Ok(input_event.map(InternalEvent::Event))
}

pub(crate) fn next_parsed<T>(iter: &mut dyn Iterator<Item = &str>) -> io::Result<T>
where
    T: std::str::FromStr,
{
    iter.next()
        .ok_or_else(could_not_parse_event_error)?
        .parse::<T>()
        .map_err(|_| could_not_parse_event_error())
}

fn modifier_and_kind_parsed(iter: &mut dyn Iterator<Item = &str>) -> io::Result<(u8, u8)> {
    let mut sub_split = iter
        .next()
        .ok_or_else(could_not_parse_event_error)?
        .split(':');

    let modifier_mask = next_parsed::<u8>(&mut sub_split)?;

    if let Ok(kind_code) = next_parsed::<u8>(&mut sub_split) {
        Ok((modifier_mask, kind_code))
    } else {
        Ok((modifier_mask, 1))
    }
}

pub(crate) fn parse_csi_cursor_position(buffer: &[u8]) -> io::Result<Option<InternalEvent>> {
    // ESC [ Cy ; Cx R
    //   Cy - cursor row number (starting from 1)
    //   Cx - cursor column number (starting from 1)
    assert!(buffer.starts_with(b"\x1B[")); // ESC [
    assert!(buffer.ends_with(b"R"));

    let s = std::str::from_utf8(&buffer[2..buffer.len() - 1])
        .map_err(|_| could_not_parse_event_error())?;

    let mut split = s.split(';');

    let y = next_parsed::<u16>(&mut split)? - 1;
    let x = next_parsed::<u16>(&mut split)? - 1;

    Ok(Some(InternalEvent::CursorPosition(x, y)))
}

fn parse_csi_keyboard_enhancement_flags(buffer: &[u8]) -> io::Result<Option<InternalEvent>> {
    // ESC [ ? flags u
    assert!(buffer.starts_with(b"\x1B[?")); // ESC [ ?
    assert!(buffer.ends_with(b"u"));

    if buffer.len() < 5 {
        return Ok(None);
    }

    let bits = buffer[3];
    let mut flags = KeyboardEnhancementFlags::empty();

    if bits & 1 != 0 {
        flags |= KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES;
    }
    if bits & 2 != 0 {
        flags |= KeyboardEnhancementFlags::REPORT_EVENT_TYPES;
    }
    if bits & 4 != 0 {
        flags |= KeyboardEnhancementFlags::REPORT_ALTERNATE_KEYS;
    }
    if bits & 8 != 0 {
        flags |= KeyboardEnhancementFlags::REPORT_ALL_KEYS_AS_ESCAPE_CODES;
    }
    // *Note*: this is not yet supported by crossterm.
    // if bits & 16 != 0 {
    //     flags |= KeyboardEnhancementFlags::REPORT_ASSOCIATED_TEXT;
    // }

    Ok(Some(InternalEvent::KeyboardEnhancementFlags(flags)))
}

fn parse_csi_primary_device_attributes(buffer: &[u8]) -> io::Result<Option<InternalEvent>> {
    // ESC [ 64 ; attr1 ; attr2 ; ... ; attrn ; c
    assert!(buffer.starts_with(b"\x1B[?"));
    assert!(buffer.ends_with(b"c"));

    // This is a stub for parsing the primary device attributes. This response is not
    // exposed in the crossterm API so we don't need to parse the individual attributes yet.
    // See <https://vt100.net/docs/vt510-rm/DA1.html>

    Ok(Some(InternalEvent::PrimaryDeviceAttributes))
}

fn parse_modifiers(mask: u8) -> KeyModifiers {
    let modifier_mask = mask.saturating_sub(1);
    let mut modifiers = KeyModifiers::empty();
    if modifier_mask & 1 != 0 {
        modifiers |= KeyModifiers::SHIFT;
    }
    if modifier_mask & 2 != 0 {
        modifiers |= KeyModifiers::ALT;
    }
    if modifier_mask & 4 != 0 {
        modifiers |= KeyModifiers::CONTROL;
    }
    if modifier_mask & 8 != 0 {
        modifiers |= KeyModifiers::SUPER;
    }
    if modifier_mask & 16 != 0 {
        modifiers |= KeyModifiers::HYPER;
    }
    if modifier_mask & 32 != 0 {
        modifiers |= KeyModifiers::META;
    }
    modifiers
}

fn parse_modifiers_to_state(mask: u8) -> KeyEventState {
    let modifier_mask = mask.saturating_sub(1);
    let mut state = KeyEventState::empty();
    if modifier_mask & 64 != 0 {
        state |= KeyEventState::CAPS_LOCK;
    }
    if modifier_mask & 128 != 0 {
        state |= KeyEventState::NUM_LOCK;
    }
    state
}

fn parse_key_event_kind(kind: u8) -> KeyEventKind {
    match kind {
        1 => KeyEventKind::Press,
        2 => KeyEventKind::Repeat,
        3 => KeyEventKind::Release,
        _ => KeyEventKind::Press,
    }
}

pub(crate) fn parse_csi_modifier_key_code(buffer: &[u8]) -> io::Result<Option<InternalEvent>> {
    assert!(buffer.starts_with(b"\x1B[")); // ESC [
                                           //
    let s = std::str::from_utf8(&buffer[2..buffer.len() - 1])
        .map_err(|_| could_not_parse_event_error())?;
    let mut split = s.split(';');

    split.next();

    let (modifiers, kind) =
        if let Ok((modifier_mask, kind_code)) = modifier_and_kind_parsed(&mut split) {
            (
                parse_modifiers(modifier_mask),
                parse_key_event_kind(kind_code),
            )
        } else if buffer.len() > 3 {
            (
                parse_modifiers(
                    (buffer[buffer.len() - 2] as char)
                        .to_digit(10)
                        .ok_or_else(could_not_parse_event_error)? as u8,
                ),
                KeyEventKind::Press,
            )
        } else {
            (KeyModifiers::NONE, KeyEventKind::Press)
        };
    let key = buffer[buffer.len() - 1];

    let keycode = match key {
        b'A' => KeyCode::Up,
        b'B' => KeyCode::Down,
        b'C' => KeyCode::Right,
        b'D' => KeyCode::Left,
        b'F' => KeyCode::End,
        b'H' => KeyCode::Home,
        b'P' => KeyCode::F(1),
        b'Q' => KeyCode::F(2),
        b'R' => KeyCode::F(3),
        b'S' => KeyCode::F(4),
        _ => return Err(could_not_parse_event_error()),
    };

    let input_event = Event::Key(KeyEvent::new_with_kind(keycode, modifiers, kind));

    Ok(Some(InternalEvent::Event(input_event)))
}

fn translate_functional_key_code(codepoint: u32) -> Option<(KeyCode, KeyEventState)> {
    if let Some(keycode) = match codepoint {
        57399 => Some(KeyCode::Char('0')),
        57400 => Some(KeyCode::Char('1')),
        57401 => Some(KeyCode::Char('2')),
        57402 => Some(KeyCode::Char('3')),
        57403 => Some(KeyCode::Char('4')),
        57404 => Some(KeyCode::Char('5')),
        57405 => Some(KeyCode::Char('6')),
        57406 => Some(KeyCode::Char('7')),
        57407 => Some(KeyCode::Char('8')),
        57408 => Some(KeyCode::Char('9')),
        57409 => Some(KeyCode::Char('.')),
        57410 => Some(KeyCode::Char('/')),
        57411 => Some(KeyCode::Char('*')),
        57412 => Some(KeyCode::Char('-')),
        57413 => Some(KeyCode::Char('+')),
        57414 => Some(KeyCode::Enter),
        57415 => Some(KeyCode::Char('=')),
        57416 => Some(KeyCode::Char(',')),
        57417 => Some(KeyCode::Left),
        57418 => Some(KeyCode::Right),
        57419 => Some(KeyCode::Up),
        57420 => Some(KeyCode::Down),
        57421 => Some(KeyCode::PageUp),
        57422 => Some(KeyCode::PageDown),
        57423 => Some(KeyCode::Home),
        57424 => Some(KeyCode::End),
        57425 => Some(KeyCode::Insert),
        57426 => Some(KeyCode::Delete),
        57427 => Some(KeyCode::KeypadBegin),
        _ => None,
    } {
        return Some((keycode, KeyEventState::KEYPAD));
    }

    if let Some(keycode) = match codepoint {
        57358 => Some(KeyCode::CapsLock),
        57359 => Some(KeyCode::ScrollLock),
        57360 => Some(KeyCode::NumLock),
        57361 => Some(KeyCode::PrintScreen),
        57362 => Some(KeyCode::Pause),
        57363 => Some(KeyCode::Menu),
        57376 => Some(KeyCode::F(13)),
        57377 => Some(KeyCode::F(14)),
        57378 => Some(KeyCode::F(15)),
        57379 => Some(KeyCode::F(16)),
        57380 => Some(KeyCode::F(17)),
        57381 => Some(KeyCode::F(18)),
        57382 => Some(KeyCode::F(19)),
        57383 => Some(KeyCode::F(20)),
        57384 => Some(KeyCode::F(21)),
        57385 => Some(KeyCode::F(22)),
        57386 => Some(KeyCode::F(23)),
        57387 => Some(KeyCode::F(24)),
        57388 => Some(KeyCode::F(25)),
        57389 => Some(KeyCode::F(26)),
        57390 => Some(KeyCode::F(27)),
        57391 => Some(KeyCode::F(28)),
        57392 => Some(KeyCode::F(29)),
        57393 => Some(KeyCode::F(30)),
        57394 => Some(KeyCode::F(31)),
        57395 => Some(KeyCode::F(32)),
        57396 => Some(KeyCode::F(33)),
        57397 => Some(KeyCode::F(34)),
        57398 => Some(KeyCode::F(35)),
        57428 => Some(KeyCode::Media(MediaKeyCode::Play)),
        57429 => Some(KeyCode::Media(MediaKeyCode::Pause)),
        57430 => Some(KeyCode::Media(MediaKeyCode::PlayPause)),
        57431 => Some(KeyCode::Media(MediaKeyCode::Reverse)),
        57432 => Some(KeyCode::Media(MediaKeyCode::Stop)),
        57433 => Some(KeyCode::Media(MediaKeyCode::FastForward)),
        57434 => Some(KeyCode::Media(MediaKeyCode::Rewind)),
        57435 => Some(KeyCode::Media(MediaKeyCode::TrackNext)),
        57436 => Some(KeyCode::Media(MediaKeyCode::TrackPrevious)),
        57437 => Some(KeyCode::Media(MediaKeyCode::Record)),
        57438 => Some(KeyCode::Media(MediaKeyCode::LowerVolume)),
        57439 => Some(KeyCode::Media(MediaKeyCode::RaiseVolume)),
        57440 => Some(KeyCode::Media(MediaKeyCode::MuteVolume)),
        57441 => Some(KeyCode::Modifier(ModifierKeyCode::LeftShift)),
        57442 => Some(KeyCode::Modifier(ModifierKeyCode::LeftControl)),
        57443 => Some(KeyCode::Modifier(ModifierKeyCode::LeftAlt)),
        57444 => Some(KeyCode::Modifier(ModifierKeyCode::LeftSuper)),
        57445 => Some(KeyCode::Modifier(ModifierKeyCode::LeftHyper)),
        57446 => Some(KeyCode::Modifier(ModifierKeyCode::LeftMeta)),
        57447 => Some(KeyCode::Modifier(ModifierKeyCode::RightShift)),
        57448 => Some(KeyCode::Modifier(ModifierKeyCode::RightControl)),
        57449 => Some(KeyCode::Modifier(ModifierKeyCode::RightAlt)),
        57450 => Some(KeyCode::Modifier(ModifierKeyCode::RightSuper)),
        57451 => Some(KeyCode::Modifier(ModifierKeyCode::RightHyper)),
        57452 => Some(KeyCode::Modifier(ModifierKeyCode::RightMeta)),
        57453 => Some(KeyCode::Modifier(ModifierKeyCode::IsoLevel3Shift)),
        57454 => Some(KeyCode::Modifier(ModifierKeyCode::IsoLevel5Shift)),
        _ => None,
    } {
        return Some((keycode, KeyEventState::empty()));
    }

    None
}

pub(crate) fn parse_csi_u_encoded_key_code(buffer: &[u8]) -> io::Result<Option<InternalEvent>> {
    assert!(buffer.starts_with(b"\x1B[")); // ESC [
    assert!(buffer.ends_with(b"u"));

    // This function parses `CSI … u` sequences. These are sequences defined in either
    // the `CSI u` (a.k.a. "Fix Keyboard Input on Terminals - Please", https://www.leonerd.org.uk/hacks/fixterms/)
    // or Kitty Keyboard Protocol (https://sw.kovidgoyal.net/kitty/keyboard-protocol/) specifications.
    // This CSI sequence is a tuple of semicolon-separated numbers.
    let s = std::str::from_utf8(&buffer[2..buffer.len() - 1])
        .map_err(|_| could_not_parse_event_error())?;
    let mut split = s.split(';');

    // In `CSI u`, this is parsed as:
    //
    //     CSI codepoint ; modifiers u
    //     codepoint: ASCII Dec value
    //
    // The Kitty Keyboard Protocol extends this with optional components that can be
    // enabled progressively. The full sequence is parsed as:
    //
    //     CSI unicode-key-code:alternate-key-codes ; modifiers:event-type ; text-as-codepoints u
    let mut codepoints = split
        .next()
        .ok_or_else(could_not_parse_event_error)?
        .split(':');

    let codepoint = codepoints
        .next()
        .ok_or_else(could_not_parse_event_error)?
        .parse::<u32>()
        .map_err(|_| could_not_parse_event_error())?;

    let (mut modifiers, kind, state_from_modifiers) =
        if let Ok((modifier_mask, kind_code)) = modifier_and_kind_parsed(&mut split) {
            (
                parse_modifiers(modifier_mask),
                parse_key_event_kind(kind_code),
                parse_modifiers_to_state(modifier_mask),
            )
        } else {
            (KeyModifiers::NONE, KeyEventKind::Press, KeyEventState::NONE)
        };

    let (mut keycode, state_from_keycode) = {
        if let Some((special_key_code, state)) = translate_functional_key_code(codepoint) {
            (special_key_code, state)
        } else if let Some(c) = char::from_u32(codepoint) {
            (
                match c {
                    '\x1B' => KeyCode::Esc,
                    '\r' => KeyCode::Enter,
                    // Issue #371: \n = 0xA, which is also the keycode for Ctrl+J. The only reason we get
                    // newlines as input is because the terminal converts \r into \n for us. When we
                    // enter raw mode, we disable that, so \n no longer has any meaning - it's better to
                    // use Ctrl+J. Waiting to handle it here means it gets picked up later
                    '\n' if !crate::terminal::sys::is_raw_mode_enabled() => KeyCode::Enter,
                    '\t' => {
                        if modifiers.contains(KeyModifiers::SHIFT) {
                            KeyCode::BackTab
                        } else {
                            KeyCode::Tab
                        }
                    }
                    '\x7F' => KeyCode::Backspace,
                    _ => KeyCode::Char(c),
                },
                KeyEventState::empty(),
            )
        } else {
            return Err(could_not_parse_event_error());
        }
    };

    if let KeyCode::Modifier(modifier_keycode) = keycode {
        match modifier_keycode {
            ModifierKeyCode::LeftAlt | ModifierKeyCode::RightAlt => {
                modifiers.set(KeyModifiers::ALT, true)
            }
            ModifierKeyCode::LeftControl | ModifierKeyCode::RightControl => {
                modifiers.set(KeyModifiers::CONTROL, true)
            }
            ModifierKeyCode::LeftShift | ModifierKeyCode::RightShift => {
                modifiers.set(KeyModifiers::SHIFT, true)
            }
            ModifierKeyCode::LeftSuper | ModifierKeyCode::RightSuper => {
                modifiers.set(KeyModifiers::SUPER, true)
            }
            ModifierKeyCode::LeftHyper | ModifierKeyCode::RightHyper => {
                modifiers.set(KeyModifiers::HYPER, true)
            }
            ModifierKeyCode::LeftMeta | ModifierKeyCode::RightMeta => {
                modifiers.set(KeyModifiers::META, true)
            }
            _ => {}
        }
    }

    // When the "report alternate keys" flag is enabled in the Kitty Keyboard Protocol
    // and the terminal sends a keyboard event containing shift, the sequence will
    // contain an additional codepoint separated by a ':' character which contains
    // the shifted character according to the keyboard layout.
    if modifiers.contains(KeyModifiers::SHIFT) {
        if let Some(shifted_c) = codepoints
            .next()
            .and_then(|codepoint| codepoint.parse::<u32>().ok())
            .and_then(char::from_u32)
        {
            keycode = KeyCode::Char(shifted_c);
            modifiers.set(KeyModifiers::SHIFT, false);
        }
    }

    let input_event = Event::Key(KeyEvent::new_with_kind_and_state(
        keycode,
        modifiers,
        kind,
        state_from_keycode | state_from_modifiers,
    ));

    Ok(Some(InternalEvent::Event(input_event)))
}

pub(crate) fn parse_csi_special_key_code(buffer: &[u8]) -> io::Result<Option<InternalEvent>> {
    assert!(buffer.starts_with(b"\x1B[")); // ESC [
    assert!(buffer.ends_with(b"~"));

    let s = std::str::from_utf8(&buffer[2..buffer.len() - 1])
        .map_err(|_| could_not_parse_event_error())?;
    let mut split = s.split(';');

    // This CSI sequence can be a list of semicolon-separated numbers.
    let first = next_parsed::<u8>(&mut split)?;

    let (modifiers, kind, state) =
        if let Ok((modifier_mask, kind_code)) = modifier_and_kind_parsed(&mut split) {
            (
                parse_modifiers(modifier_mask),
                parse_key_event_kind(kind_code),
                parse_modifiers_to_state(modifier_mask),
            )
        } else {
            (KeyModifiers::NONE, KeyEventKind::Press, KeyEventState::NONE)
        };

    let keycode = match first {
        1 | 7 => KeyCode::Home,
        2 => KeyCode::Insert,
        3 => KeyCode::Delete,
        4 | 8 => KeyCode::End,
        5 => KeyCode::PageUp,
        6 => KeyCode::PageDown,
        v @ 11..=15 => KeyCode::F(v - 10),
        v @ 17..=21 => KeyCode::F(v - 11),
        v @ 23..=26 => KeyCode::F(v - 12),
        v @ 28..=29 => KeyCode::F(v - 15),
        v @ 31..=34 => KeyCode::F(v - 17),
        _ => return Err(could_not_parse_event_error()),
    };

    let input_event = Event::Key(KeyEvent::new_with_kind_and_state(
        keycode, modifiers, kind, state,
    ));

    Ok(Some(InternalEvent::Event(input_event)))
}

pub(crate) fn parse_csi_rxvt_mouse(buffer: &[u8]) -> io::Result<Option<InternalEvent>> {
    // rxvt mouse encoding:
    // ESC [ Cb ; Cx ; Cy ; M

    assert!(buffer.starts_with(b"\x1B[")); // ESC [
    assert!(buffer.ends_with(b"M"));

    let s = std::str::from_utf8(&buffer[2..buffer.len() - 1])
        .map_err(|_| could_not_parse_event_error())?;
    let mut split = s.split(';');

    let cb = next_parsed::<u8>(&mut split)?
        .checked_sub(32)
        .ok_or_else(could_not_parse_event_error)?;
    let (kind, modifiers) = parse_cb(cb)?;

    let cx = next_parsed::<u16>(&mut split)? - 1;
    let cy = next_parsed::<u16>(&mut split)? - 1;

    Ok(Some(InternalEvent::Event(Event::Mouse(MouseEvent {
        kind,
        column: cx,
        row: cy,
        modifiers,
    }))))
}

pub(crate) fn parse_csi_normal_mouse(buffer: &[u8]) -> io::Result<Option<InternalEvent>> {
    // Normal mouse encoding: ESC [ M CB Cx Cy (6 characters only).

    assert!(buffer.starts_with(b"\x1B[M")); // ESC [ M

    if buffer.len() < 6 {
        return Ok(None);
    }

    let cb = buffer[3]
        .checked_sub(32)
        .ok_or_else(could_not_parse_event_error)?;
    let (kind, modifiers) = parse_cb(cb)?;

    // See http://www.xfree86.org/current/ctlseqs.html#Mouse%20Tracking
    // The upper left character position on the terminal is denoted as 1,1.
    // Subtract 1 to keep it synced with cursor
    let cx = u16::from(buffer[4].saturating_sub(32)) - 1;
    let cy = u16::from(buffer[5].saturating_sub(32)) - 1;

    Ok(Some(InternalEvent::Event(Event::Mouse(MouseEvent {
        kind,
        column: cx,
        row: cy,
        modifiers,
    }))))
}

pub(crate) fn parse_csi_sgr_mouse(buffer: &[u8]) -> io::Result<Option<InternalEvent>> {
    // ESC [ < Cb ; Cx ; Cy (;) (M or m)

    assert!(buffer.starts_with(b"\x1B[<")); // ESC [ <

    if !buffer.ends_with(b"m") && !buffer.ends_with(b"M") {
        return Ok(None);
    }

    let s = std::str::from_utf8(&buffer[3..buffer.len() - 1])
        .map_err(|_| could_not_parse_event_error())?;
    let mut split = s.split(';');

    let cb = next_parsed::<u8>(&mut split)?;
    let (kind, modifiers) = parse_cb(cb)?;

    // See http://www.xfree86.org/current/ctlseqs.html#Mouse%20Tracking
    // The upper left character position on the terminal is denoted as 1,1.
    // Subtract 1 to keep it synced with cursor
    let cx = next_parsed::<u16>(&mut split)? - 1;
    let cy = next_parsed::<u16>(&mut split)? - 1;

    // When button 3 in Cb is used to represent mouse release, you can't tell which button was
    // released. SGR mode solves this by having the sequence end with a lowercase m if it's a
    // button release and an uppercase M if it's a button press.
    //
    // We've already checked that the last character is a lowercase or uppercase M at the start of
    // this function, so we just need one if.
    let kind = if buffer.last() == Some(&b'm') {
        match kind {
            MouseEventKind::Down(button) => MouseEventKind::Up(button),
            other => other,
        }
    } else {
        kind
    };

    Ok(Some(InternalEvent::Event(Event::Mouse(MouseEvent {
        kind,
        column: cx,
        row: cy,
        modifiers,
    }))))
}

/// Cb is the byte of a mouse input that contains the button being used, the key modifiers being
/// held and whether the mouse is dragging or not.
///
/// Bit layout of cb, from low to high:
///
/// - button number
/// - button number
/// - shift
/// - meta (alt)
/// - control
/// - mouse is dragging
/// - button number
/// - button number
fn parse_cb(cb: u8) -> io::Result<(MouseEventKind, KeyModifiers)> {
    let button_number = (cb & 0b0000_0011) | ((cb & 0b1100_0000) >> 4);
    let dragging = cb & 0b0010_0000 == 0b0010_0000;

    let kind = match (button_number, dragging) {
        (0, false) => MouseEventKind::Down(MouseButton::Left),
        (1, false) => MouseEventKind::Down(MouseButton::Middle),
        (2, false) => MouseEventKind::Down(MouseButton::Right),
        (0, true) => MouseEventKind::Drag(MouseButton::Left),
        (1, true) => MouseEventKind::Drag(MouseButton::Middle),
        (2, true) => MouseEventKind::Drag(MouseButton::Right),
        (3, false) => MouseEventKind::Up(MouseButton::Left),
        (3, true) | (4, true) | (5, true) => MouseEventKind::Moved,
        (4, false) => MouseEventKind::ScrollUp,
        (5, false) => MouseEventKind::ScrollDown,
        (6, false) => MouseEventKind::ScrollLeft,
        (7, false) => MouseEventKind::ScrollRight,
        // ttyak: the side buttons (SGR 128 and 129; a drag adds 32), then buttons 10 and 11.
        (8, false) => MouseEventKind::Down(MouseButton::Back),
        (9, false) => MouseEventKind::Down(MouseButton::Forward),
        (8, true) => MouseEventKind::Drag(MouseButton::Back),
        (9, true) => MouseEventKind::Drag(MouseButton::Forward),
        // ttyak: buttons 10 and 11 (SGR 130 and 131; a drag adds 32).
        (10, false) => MouseEventKind::Down(MouseButton::Button10),
        (11, false) => MouseEventKind::Down(MouseButton::Button11),
        (10, true) => MouseEventKind::Drag(MouseButton::Button10),
        (11, true) => MouseEventKind::Drag(MouseButton::Button11),
        // We do not support other buttons.
        _ => return Err(could_not_parse_event_error()),
    };

    let mut modifiers = KeyModifiers::empty();

    if cb & 0b0000_0100 == 0b0000_0100 {
        modifiers |= KeyModifiers::SHIFT;
    }
    if cb & 0b0000_1000 == 0b0000_1000 {
        modifiers |= KeyModifiers::ALT;
    }
    if cb & 0b0001_0000 == 0b0001_0000 {
        modifiers |= KeyModifiers::CONTROL;
    }

    Ok((kind, modifiers))
}

#[cfg(feature = "bracketed-paste")]
pub(crate) fn parse_csi_bracketed_paste(buffer: &[u8]) -> io::Result<Option<InternalEvent>> {
    // ESC [ 2 0 0 ~ pasted text ESC 2 0 1 ~
    assert!(buffer.starts_with(b"\x1B[200~"));

    if !buffer.ends_with(b"\x1b[201~") {
        Ok(None)
    } else {
        let paste = String::from_utf8_lossy(&buffer[6..buffer.len() - 6]).to_string();
        Ok(Some(InternalEvent::Event(Event::Paste(paste))))
    }
}

pub(crate) fn parse_utf8_char(buffer: &[u8]) -> io::Result<Option<char>> {
    match std::str::from_utf8(buffer) {
        Ok(s) => {
            let ch = s.chars().next().ok_or_else(could_not_parse_event_error)?;

            Ok(Some(ch))
        }
        Err(_) => {
            // from_utf8 failed, but we have to check if we need more bytes for code point
            // and if all the bytes we have no are valid

            let required_bytes = match buffer[0] {
                // https://en.wikipedia.org/wiki/UTF-8#Description
                (0x00..=0x7F) => 1, // 0xxxxxxx
                (0xC0..=0xDF) => 2, // 110xxxxx 10xxxxxx
                (0xE0..=0xEF) => 3, // 1110xxxx 10xxxxxx 10xxxxxx
                (0xF0..=0xF7) => 4, // 11110xxx 10xxxxxx 10xxxxxx 10xxxxxx
                (0x80..=0xBF) | (0xF8..=0xFF) => return Err(could_not_parse_event_error()),
            };

            // More than 1 byte, check them for 10xxxxxx pattern
            if required_bytes > 1 && buffer.len() > 1 {
                for byte in &buffer[1..] {
                    if byte & !0b0011_1111 != 0b1000_0000 {
                        return Err(could_not_parse_event_error());
                    }
                }
            }

            if buffer.len() < required_bytes {
                // All bytes looks good so far, but we need more of them
                Ok(None)
            } else {
                Err(could_not_parse_event_error())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::event::{KeyEventState, KeyModifiers, MouseButton, MouseEvent};

    use super::*;

    #[test]
    fn test_esc_key() {
        assert_eq!(
            parse_event(b"\x1B", false).unwrap(),
            Some(InternalEvent::Event(Event::Key(KeyCode::Esc.into()))),
        );
    }

    #[test]
    fn test_possible_esc_sequence() {
        assert_eq!(parse_event(b"\x1B", true).unwrap(), None,);
    }

    #[test]
    fn test_alt_key() {
        assert_eq!(
            parse_event(b"\x1Bc", false).unwrap(),
            Some(InternalEvent::Event(Event::Key(KeyEvent::new(
                KeyCode::Char('c'),
                KeyModifiers::ALT
            )))),
        );
    }

    #[test]
    fn test_alt_shift() {
        assert_eq!(
            parse_event(b"\x1BH", false).unwrap(),
            Some(InternalEvent::Event(Event::Key(KeyEvent::new(
                KeyCode::Char('H'),
                KeyModifiers::ALT | KeyModifiers::SHIFT
            )))),
        );
    }

    #[test]
    fn test_alt_ctrl() {
        assert_eq!(
            parse_event(b"\x1B\x14", false).unwrap(),
            Some(InternalEvent::Event(Event::Key(KeyEvent::new(
                KeyCode::Char('t'),
                KeyModifiers::ALT | KeyModifiers::CONTROL
            )))),
        );
    }

    #[test]
    fn test_parse_event_subsequent_calls() {
        // The main purpose of this test is to check if we're passing
        // correct slice to other parse_ functions.

        // parse_csi_cursor_position
        assert_eq!(
            parse_event(b"\x1B[20;10R", false).unwrap(),
            Some(InternalEvent::CursorPosition(9, 19))
        );

        // parse_csi
        assert_eq!(
            parse_event(b"\x1B[D", false).unwrap(),
            Some(InternalEvent::Event(Event::Key(KeyCode::Left.into()))),
        );

        // parse_csi_modifier_key_code
        assert_eq!(
            parse_event(b"\x1B[2D", false).unwrap(),
            Some(InternalEvent::Event(Event::Key(KeyEvent::new(
                KeyCode::Left,
                KeyModifiers::SHIFT
            ))))
        );

        // parse_csi_special_key_code
        assert_eq!(
            parse_event(b"\x1B[3~", false).unwrap(),
            Some(InternalEvent::Event(Event::Key(KeyCode::Delete.into()))),
        );

        // parse_csi_bracketed_paste
        #[cfg(feature = "bracketed-paste")]
        assert_eq!(
            parse_event(b"\x1B[200~on and on and on\x1B[201~", false).unwrap(),
            Some(InternalEvent::Event(Event::Paste(
                "on and on and on".to_string()
            ))),
        );

        // parse_csi_rxvt_mouse
        assert_eq!(
            parse_event(b"\x1B[32;30;40;M", false).unwrap(),
            Some(InternalEvent::Event(Event::Mouse(MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column: 29,
                row: 39,
                modifiers: KeyModifiers::empty(),
            })))
        );

        // parse_csi_normal_mouse
        assert_eq!(
            parse_event(b"\x1B[M0\x60\x70", false).unwrap(),
            Some(InternalEvent::Event(Event::Mouse(MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column: 63,
                row: 79,
                modifiers: KeyModifiers::CONTROL,
            })))
        );

        // parse_csi_sgr_mouse
        assert_eq!(
            parse_event(b"\x1B[<0;20;10;M", false).unwrap(),
            Some(InternalEvent::Event(Event::Mouse(MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column: 19,
                row: 9,
                modifiers: KeyModifiers::empty(),
            })))
        );

        // parse_utf8_char
        assert_eq!(
            parse_event("Ž".as_bytes(), false).unwrap(),
            Some(InternalEvent::Event(Event::Key(KeyEvent::new(
                KeyCode::Char('Ž'),
                KeyModifiers::SHIFT
            )))),
        );
    }

    #[test]
    fn test_parse_event() {
        assert_eq!(
            parse_event(b"\t", false).unwrap(),
            Some(InternalEvent::Event(Event::Key(KeyCode::Tab.into()))),
        );
    }

    #[test]
    fn test_parse_csi_cursor_position() {
        assert_eq!(
            parse_csi_cursor_position(b"\x1B[20;10R").unwrap(),
            Some(InternalEvent::CursorPosition(9, 19))
        );
    }

    #[test]
    fn test_parse_csi() {
        assert_eq!(
            parse_csi(b"\x1B[D").unwrap(),
            Some(InternalEvent::Event(Event::Key(KeyCode::Left.into()))),
        );
    }

    #[test]
    fn test_parse_csi_modifier_key_code() {
        assert_eq!(
            parse_csi_modifier_key_code(b"\x1B[2D").unwrap(),
            Some(InternalEvent::Event(Event::Key(KeyEvent::new(
                KeyCode::Left,
                KeyModifiers::SHIFT
            )))),
        );
    }

    #[test]
    fn test_parse_csi_special_key_code() {
        assert_eq!(
            parse_csi_special_key_code(b"\x1B[3~").unwrap(),
            Some(InternalEvent::Event(Event::Key(KeyCode::Delete.into()))),
        );
    }

    #[test]
    fn test_parse_csi_special_key_code_multiple_values_not_supported() {
        assert_eq!(
            parse_csi_special_key_code(b"\x1B[3;2~").unwrap(),
            Some(InternalEvent::Event(Event::Key(KeyEvent::new(
                KeyCode::Delete,
                KeyModifiers::SHIFT
            )))),
        );
    }

    #[cfg(feature = "bracketed-paste")]
    #[test]
    fn test_parse_csi_bracketed_paste() {
        //
        assert_eq!(
            parse_event(b"\x1B[200~o", false).unwrap(),
            None,
            "A partial bracketed paste isn't parsed"
        );
        assert_eq!(
            parse_event(b"\x1B[200~o\x1B[2D", false).unwrap(),
            None,
            "A partial bracketed paste containing another escape code isn't parsed"
        );
        assert_eq!(
            parse_event(b"\x1B[200~o\x1B[2D\x1B[201~", false).unwrap(),
            Some(InternalEvent::Event(Event::Paste("o\x1B[2D".to_string())))
        );
    }

    #[test]
    fn test_parse_csi_focus() {
        assert_eq!(
            parse_csi(b"\x1B[O").unwrap(),
            Some(InternalEvent::Event(Event::FocusLost))
        );
    }

    #[test]
    fn test_parse_csi_rxvt_mouse() {
        assert_eq!(
            parse_csi_rxvt_mouse(b"\x1B[32;30;40;M").unwrap(),
            Some(InternalEvent::Event(Event::Mouse(MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column: 29,
                row: 39,
                modifiers: KeyModifiers::empty(),
            })))
        );
    }

    #[test]
    fn test_parse_csi_normal_mouse() {
        assert_eq!(
            parse_csi_normal_mouse(b"\x1B[M0\x60\x70").unwrap(),
            Some(InternalEvent::Event(Event::Mouse(MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column: 63,
                row: 79,
                modifiers: KeyModifiers::CONTROL,
            })))
        );
    }

    // ttyak: an OSC reply (kitty's notification click) is an event, ended by BEL or ST.
    #[test]
    fn osc_replies_become_events() {
        let osc = |s: &str| Some(InternalEvent::Event(Event::Osc(s.to_string())));
        for (input, more) in [
            (&b"\x1B]99;i=w0-C1;\x1B\\"[..], false),
            (&b"\x1B]99;i=w0-C1;\x07"[..], false),
            (&b"\x1B]99;i=w0-C1;\x07"[..], true),
        ] {
            assert_eq!(parse_event(input, more).unwrap(), osc("99;i=w0-C1;"));
        }
    }

    // ttyak: `ESC ]` with nothing after stays Alt+], and so does `ESC ]` and a byte that cannot go on an OSC.
    #[test]
    fn alt_bracket_is_still_a_key() {
        let alt_bracket = Some(InternalEvent::Event(Event::Key(KeyEvent::new(
            KeyCode::Char(']'),
            KeyModifiers::ALT,
        ))));
        assert_eq!(parse_event(b"\x1B]", false).unwrap(), alt_bracket);
        assert_eq!(parse_event(b"\x1B]x", false).unwrap(), alt_bracket);
        // With more input pending it waits for the byte that tells.
        assert_eq!(parse_event(b"\x1B]", true).unwrap(), None);
        assert!(osc_broken_by(b"\x1B]", b'x'));
        assert!(osc_broken_by(b"\x1B]", b'\x1B'));
        assert!(osc_broken_by(b"\x1B]", b'1'));
        assert!(!osc_broken_by(b"\x1B]", b'9'));
        assert!(osc_broken_by(b"\x1B]9", b'x'));
        assert!(osc_broken_by(b"\x1B]99", b'x'));
        assert!(!osc_broken_by(b"\x1B]99", b';'));
    }

    // ttyak: an OSC split across reads waits for its end; what a reply cannot hold gives it up.
    #[test]
    fn an_osc_split_across_reads_waits_for_its_end() {
        assert_eq!(parse_event(b"\x1B]99;i=w0", true).unwrap(), None);
        assert_eq!(parse_event(b"\x1B]99;i=w0\x1B", true).unwrap(), None);
        assert_eq!(
            parse_event(b"\x1B]99;i=w0\x07", false).unwrap(),
            Some(InternalEvent::Event(Event::Osc("99;i=w0".to_string())))
        );
        // Given up: a space, a control, a non-ASCII byte, an ESC not followed by `\`, the length cap.
        for next in [b' ', b'\r', b'\n', b'\t', 0x7F, 0xC3] {
            assert!(osc_broken_by(b"\x1B]99;i=w0", next), "{next:#x}");
        }
        assert!(osc_broken_by(b"\x1B]99;i=w0\x1B", b'['));
        assert!(!osc_broken_by(b"\x1B]99;i=w0\x1B", b'\\'));
        let mut long = b"\x1B]99;".to_vec();
        long.resize(OSC_MAX_LEN - 1, b'a');
        assert!(!osc_broken_by(&long, b'a'));
        long.push(b'a');
        assert!(osc_broken_by(&long, b'a'));
    }

    fn run(reads: &[(&[u8], bool)]) -> Vec<InternalEvent> {
        let (mut buffer, mut events) = (Vec::new(), VecDeque::new());
        for (input, more) in reads {
            advance_bytes(&mut buffer, &mut events, input, *more);
        }
        events.into_iter().collect()
    }

    fn key(c: char, modifiers: KeyModifiers) -> InternalEvent {
        InternalEvent::Event(Event::Key(KeyEvent::new(KeyCode::Char(c), modifiers)))
    }

    // ttyak: through the readers' loop: nothing a stray OSC held is lost, and what follows it stays what it is.
    #[test]
    fn a_stray_osc_gives_back_what_it_held() {
        let alt = KeyModifiers::ALT;
        let none = KeyModifiers::NONE;
        // `ESC ] x`: Alt+] then `x`.
        assert_eq!(run(&[(b"\x1B]x", false)]), vec![key(']', alt), key('x', none)]);
        // The ST split between ESC and `\` across reads.
        assert_eq!(
            run(&[(b"\x1B]99;i=w0;\x1B", true), (b"\\", false)]),
            vec![InternalEvent::Event(Event::Osc("99;i=w0;".to_string()))]
        );
        // A stray OSC, then typing: Alt+], the digits and `;` as keys, then `hello`.
        let mut want = vec![key(']', alt)];
        want.extend("99;hello".chars().map(|c| key(c, none)));
        assert_eq!(run(&[(b"\x1B]99;hello", false)]), want);
        // An Enter inside it is an Enter.
        assert_eq!(
            run(&[(&b"\x1B]99;\r"[..], false)])[4],
            InternalEvent::Event(Event::Key(KeyCode::Enter.into()))
        );
        // A stray OSC, then a mouse move, a focus report, an arrow.
        for (seq, ev) in [
            (&b"\x1B[<35;10;5M"[..], Event::Mouse(MouseEvent { kind: MouseEventKind::Moved, column: 9, row: 4, modifiers: KeyModifiers::NONE })),
            (&b"\x1B[I"[..], Event::FocusGained),
            (&b"\x1B[A"[..], Event::Key(KeyCode::Up.into())),
        ] {
            let mut input = b"\x1B]99;".to_vec();
            input.extend_from_slice(seq);
            let got = run(&[(&input, false)]);
            assert_eq!(got.last(), Some(&InternalEvent::Event(ev)), "{seq:?}");
            assert_eq!(got.len(), 1 + 3 + 1, "{seq:?}: Alt+], `99;`, the event");
        }
    }

    // ttyak: the length cap, through the readers' loop: a reply of `OSC_MAX_LEN` bytes, its end included, is one OSC;
    // a byte more and it is given back as input (the parser itself never sees a longer one).
    #[test]
    fn the_longest_reply_is_read_and_a_longer_one_given_back() {
        let reply = |len: usize| {
            let mut bytes = b"\x1B]99;".to_vec();
            bytes.resize(len - 1, b'a');
            bytes.push(b'\x07');
            bytes
        };
        let longest = reply(OSC_MAX_LEN);
        let payload = String::from_utf8(longest[2..longest.len() - 1].to_vec()).unwrap();
        assert_eq!(run(&[(&longest, false)]), vec![InternalEvent::Event(Event::Osc(payload))]);
        let got = run(&[(&reply(OSC_MAX_LEN + 1), false)]);
        assert_eq!(got[0], key(']', KeyModifiers::ALT));
        assert_eq!(got[1..4], [key('9', KeyModifiers::NONE), key('9', KeyModifiers::NONE), key(';', KeyModifiers::NONE)]);
        assert!(!got.iter().any(|e| matches!(e, InternalEvent::Event(Event::Osc(_)))), "{got:?}");
    }

    // ttyak: the side buttons (SGR 8 to 11) decode as Back, Forward, Button10 and Button11.
    #[test]
    fn test_parse_csi_sgr_mouse_side_buttons() {
        let ev = |kind, column, row| {
            Some(InternalEvent::Event(Event::Mouse(MouseEvent {
                kind,
                column,
                row,
                modifiers: KeyModifiers::empty(),
            })))
        };
        assert_eq!(
            parse_csi_sgr_mouse(b"\x1B[<128;10;5M").unwrap(),
            ev(MouseEventKind::Down(MouseButton::Back), 9, 4)
        );
        assert_eq!(
            parse_csi_sgr_mouse(b"\x1B[<128;10;5m").unwrap(),
            ev(MouseEventKind::Up(MouseButton::Back), 9, 4)
        );
        assert_eq!(
            parse_csi_sgr_mouse(b"\x1B[<129;10;5M").unwrap(),
            ev(MouseEventKind::Down(MouseButton::Forward), 9, 4)
        );
        assert_eq!(
            parse_csi_sgr_mouse(b"\x1B[<129;10;5m").unwrap(),
            ev(MouseEventKind::Up(MouseButton::Forward), 9, 4)
        );
        // Drag: the motion bit (32) on top of the button.
        assert_eq!(
            parse_csi_sgr_mouse(b"\x1B[<160;10;5M").unwrap(),
            ev(MouseEventKind::Drag(MouseButton::Back), 9, 4)
        );
        assert_eq!(
            parse_csi_sgr_mouse(b"\x1B[<161;10;5M").unwrap(),
            ev(MouseEventKind::Drag(MouseButton::Forward), 9, 4)
        );
        // Buttons 10 and 11 (SGR 130 and 131): down, release, drag.
        assert_eq!(
            parse_csi_sgr_mouse(b"\x1B[<130;10;5M").unwrap(),
            ev(MouseEventKind::Down(MouseButton::Button10), 9, 4)
        );
        assert_eq!(
            parse_csi_sgr_mouse(b"\x1B[<130;10;5m").unwrap(),
            ev(MouseEventKind::Up(MouseButton::Button10), 9, 4)
        );
        assert_eq!(
            parse_csi_sgr_mouse(b"\x1B[<131;10;5M").unwrap(),
            ev(MouseEventKind::Down(MouseButton::Button11), 9, 4)
        );
        assert_eq!(
            parse_csi_sgr_mouse(b"\x1B[<131;10;5m").unwrap(),
            ev(MouseEventKind::Up(MouseButton::Button11), 9, 4)
        );
        assert_eq!(
            parse_csi_sgr_mouse(b"\x1B[<162;10;5M").unwrap(),
            ev(MouseEventKind::Drag(MouseButton::Button10), 9, 4)
        );
        assert_eq!(
            parse_csi_sgr_mouse(b"\x1B[<163;10;5M").unwrap(),
            ev(MouseEventKind::Drag(MouseButton::Button11), 9, 4)
        );
        // Button 12 (SGR 192; 132 is Back with Shift) and up stay unsupported.
        assert!(parse_csi_sgr_mouse(b"\x1B[<192;10;5M").is_err());
        assert!(parse_csi_sgr_mouse(b"\x1B[<224;10;5M").is_err());
    }

    #[test]
    fn test_parse_csi_sgr_mouse() {
        assert_eq!(
            parse_csi_sgr_mouse(b"\x1B[<0;20;10;M").unwrap(),
            Some(InternalEvent::Event(Event::Mouse(MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column: 19,
                row: 9,
                modifiers: KeyModifiers::empty(),
            })))
        );
        assert_eq!(
            parse_csi_sgr_mouse(b"\x1B[<0;20;10M").unwrap(),
            Some(InternalEvent::Event(Event::Mouse(MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column: 19,
                row: 9,
                modifiers: KeyModifiers::empty(),
            })))
        );
        assert_eq!(
            parse_csi_sgr_mouse(b"\x1B[<0;20;10;m").unwrap(),
            Some(InternalEvent::Event(Event::Mouse(MouseEvent {
                kind: MouseEventKind::Up(MouseButton::Left),
                column: 19,
                row: 9,
                modifiers: KeyModifiers::empty(),
            })))
        );
        assert_eq!(
            parse_csi_sgr_mouse(b"\x1B[<0;20;10m").unwrap(),
            Some(InternalEvent::Event(Event::Mouse(MouseEvent {
                kind: MouseEventKind::Up(MouseButton::Left),
                column: 19,
                row: 9,
                modifiers: KeyModifiers::empty(),
            })))
        );
    }

    #[test]
    fn test_utf8() {
        // https://www.php.net/manual/en/reference.pcre.pattern.modifiers.php#54805

        // 'Valid ASCII' => "a",
        assert_eq!(parse_utf8_char(b"a").unwrap(), Some('a'),);

        // 'Valid 2 Octet Sequence' => "\xc3\xb1",
        assert_eq!(parse_utf8_char(&[0xC3, 0xB1]).unwrap(), Some('ñ'),);

        // 'Invalid 2 Octet Sequence' => "\xc3\x28",
        assert!(parse_utf8_char(&[0xC3, 0x28]).is_err());

        // 'Invalid Sequence Identifier' => "\xa0\xa1",
        assert!(parse_utf8_char(&[0xA0, 0xA1]).is_err());

        // 'Valid 3 Octet Sequence' => "\xe2\x82\xa1",
        assert_eq!(
            parse_utf8_char(&[0xE2, 0x81, 0xA1]).unwrap(),
            Some('\u{2061}'),
        );

        // 'Invalid 3 Octet Sequence (in 2nd Octet)' => "\xe2\x28\xa1",
        assert!(parse_utf8_char(&[0xE2, 0x28, 0xA1]).is_err());

        // 'Invalid 3 Octet Sequence (in 3rd Octet)' => "\xe2\x82\x28",
        assert!(parse_utf8_char(&[0xE2, 0x82, 0x28]).is_err());

        // 'Valid 4 Octet Sequence' => "\xf0\x90\x8c\xbc",
        assert_eq!(
            parse_utf8_char(&[0xF0, 0x90, 0x8C, 0xBC]).unwrap(),
            Some('𐌼'),
        );

        // 'Invalid 4 Octet Sequence (in 2nd Octet)' => "\xf0\x28\x8c\xbc",
        assert!(parse_utf8_char(&[0xF0, 0x28, 0x8C, 0xBC]).is_err());

        // 'Invalid 4 Octet Sequence (in 3rd Octet)' => "\xf0\x90\x28\xbc",
        assert!(parse_utf8_char(&[0xF0, 0x90, 0x28, 0xBC]).is_err());

        // 'Invalid 4 Octet Sequence (in 4th Octet)' => "\xf0\x28\x8c\x28",
        assert!(parse_utf8_char(&[0xF0, 0x28, 0x8C, 0x28]).is_err());
    }

    #[test]
    fn test_parse_char_event_lowercase() {
        assert_eq!(
            parse_event(b"c", false).unwrap(),
            Some(InternalEvent::Event(Event::Key(KeyEvent::new(
                KeyCode::Char('c'),
                KeyModifiers::empty()
            )))),
        );
    }

    #[test]
    fn test_parse_char_event_uppercase() {
        assert_eq!(
            parse_event(b"C", false).unwrap(),
            Some(InternalEvent::Event(Event::Key(KeyEvent::new(
                KeyCode::Char('C'),
                KeyModifiers::SHIFT
            )))),
        );
    }

    #[test]
    fn test_parse_basic_csi_u_encoded_key_code() {
        assert_eq!(
            parse_csi_u_encoded_key_code(b"\x1B[97u").unwrap(),
            Some(InternalEvent::Event(Event::Key(KeyEvent::new(
                KeyCode::Char('a'),
                KeyModifiers::empty()
            )))),
        );
        assert_eq!(
            parse_csi_u_encoded_key_code(b"\x1B[97;2u").unwrap(),
            Some(InternalEvent::Event(Event::Key(KeyEvent::new(
                KeyCode::Char('A'),
                KeyModifiers::SHIFT
            )))),
        );
        assert_eq!(
            parse_csi_u_encoded_key_code(b"\x1B[97;7u").unwrap(),
            Some(InternalEvent::Event(Event::Key(KeyEvent::new(
                KeyCode::Char('a'),
                KeyModifiers::ALT | KeyModifiers::CONTROL
            )))),
        );
    }

    #[test]
    fn test_parse_basic_csi_u_encoded_key_code_special_keys() {
        assert_eq!(
            parse_csi_u_encoded_key_code(b"\x1B[13u").unwrap(),
            Some(InternalEvent::Event(Event::Key(KeyEvent::new(
                KeyCode::Enter,
                KeyModifiers::empty()
            )))),
        );
        assert_eq!(
            parse_csi_u_encoded_key_code(b"\x1B[27u").unwrap(),
            Some(InternalEvent::Event(Event::Key(KeyEvent::new(
                KeyCode::Esc,
                KeyModifiers::empty()
            )))),
        );
        assert_eq!(
            parse_csi_u_encoded_key_code(b"\x1B[57358u").unwrap(),
            Some(InternalEvent::Event(Event::Key(KeyEvent::new(
                KeyCode::CapsLock,
                KeyModifiers::empty()
            )))),
        );
        assert_eq!(
            parse_csi_u_encoded_key_code(b"\x1B[57376u").unwrap(),
            Some(InternalEvent::Event(Event::Key(KeyEvent::new(
                KeyCode::F(13),
                KeyModifiers::empty()
            )))),
        );
        assert_eq!(
            parse_csi_u_encoded_key_code(b"\x1B[57428u").unwrap(),
            Some(InternalEvent::Event(Event::Key(KeyEvent::new(
                KeyCode::Media(MediaKeyCode::Play),
                KeyModifiers::empty()
            )))),
        );
        assert_eq!(
            parse_csi_u_encoded_key_code(b"\x1B[57441u").unwrap(),
            Some(InternalEvent::Event(Event::Key(KeyEvent::new(
                KeyCode::Modifier(ModifierKeyCode::LeftShift),
                KeyModifiers::SHIFT,
            )))),
        );
    }

    #[test]
    fn test_parse_csi_u_encoded_keypad_code() {
        assert_eq!(
            parse_csi_u_encoded_key_code(b"\x1B[57399u").unwrap(),
            Some(InternalEvent::Event(Event::Key(
                KeyEvent::new_with_kind_and_state(
                    KeyCode::Char('0'),
                    KeyModifiers::empty(),
                    KeyEventKind::Press,
                    KeyEventState::KEYPAD,
                )
            ))),
        );
        assert_eq!(
            parse_csi_u_encoded_key_code(b"\x1B[57419u").unwrap(),
            Some(InternalEvent::Event(Event::Key(
                KeyEvent::new_with_kind_and_state(
                    KeyCode::Up,
                    KeyModifiers::empty(),
                    KeyEventKind::Press,
                    KeyEventState::KEYPAD,
                )
            ))),
        );
    }

    #[test]
    fn test_parse_csi_u_encoded_key_code_with_types() {
        assert_eq!(
            parse_csi_u_encoded_key_code(b"\x1B[97;1u").unwrap(),
            Some(InternalEvent::Event(Event::Key(KeyEvent::new_with_kind(
                KeyCode::Char('a'),
                KeyModifiers::empty(),
                KeyEventKind::Press,
            )))),
        );
        assert_eq!(
            parse_csi_u_encoded_key_code(b"\x1B[97;1:1u").unwrap(),
            Some(InternalEvent::Event(Event::Key(KeyEvent::new_with_kind(
                KeyCode::Char('a'),
                KeyModifiers::empty(),
                KeyEventKind::Press,
            )))),
        );
        assert_eq!(
            parse_csi_u_encoded_key_code(b"\x1B[97;5:1u").unwrap(),
            Some(InternalEvent::Event(Event::Key(KeyEvent::new_with_kind(
                KeyCode::Char('a'),
                KeyModifiers::CONTROL,
                KeyEventKind::Press,
            )))),
        );
        assert_eq!(
            parse_csi_u_encoded_key_code(b"\x1B[97;1:2u").unwrap(),
            Some(InternalEvent::Event(Event::Key(KeyEvent::new_with_kind(
                KeyCode::Char('a'),
                KeyModifiers::empty(),
                KeyEventKind::Repeat,
            )))),
        );
        assert_eq!(
            parse_csi_u_encoded_key_code(b"\x1B[97;1:3u").unwrap(),
            Some(InternalEvent::Event(Event::Key(KeyEvent::new_with_kind(
                KeyCode::Char('a'),
                KeyModifiers::empty(),
                KeyEventKind::Release,
            )))),
        );
    }

    #[test]
    fn test_parse_csi_u_encoded_key_code_has_modifier_on_modifier_press() {
        assert_eq!(
            parse_csi_u_encoded_key_code(b"\x1B[57449u").unwrap(),
            Some(InternalEvent::Event(Event::Key(KeyEvent::new_with_kind(
                KeyCode::Modifier(ModifierKeyCode::RightAlt),
                KeyModifiers::ALT,
                KeyEventKind::Press,
            )))),
        );
        assert_eq!(
            parse_csi_u_encoded_key_code(b"\x1B[57449;3:3u").unwrap(),
            Some(InternalEvent::Event(Event::Key(KeyEvent::new_with_kind(
                KeyCode::Modifier(ModifierKeyCode::RightAlt),
                KeyModifiers::ALT,
                KeyEventKind::Release,
            )))),
        );
        assert_eq!(
            parse_csi_u_encoded_key_code(b"\x1B[57450u").unwrap(),
            Some(InternalEvent::Event(Event::Key(KeyEvent::new(
                KeyCode::Modifier(ModifierKeyCode::RightSuper),
                KeyModifiers::SUPER,
            )))),
        );
        assert_eq!(
            parse_csi_u_encoded_key_code(b"\x1B[57451u").unwrap(),
            Some(InternalEvent::Event(Event::Key(KeyEvent::new(
                KeyCode::Modifier(ModifierKeyCode::RightHyper),
                KeyModifiers::HYPER,
            )))),
        );
        assert_eq!(
            parse_csi_u_encoded_key_code(b"\x1B[57452u").unwrap(),
            Some(InternalEvent::Event(Event::Key(KeyEvent::new(
                KeyCode::Modifier(ModifierKeyCode::RightMeta),
                KeyModifiers::META,
            )))),
        );
    }

    #[test]
    fn test_parse_csi_u_encoded_key_code_with_extra_modifiers() {
        assert_eq!(
            parse_csi_u_encoded_key_code(b"\x1B[97;9u").unwrap(),
            Some(InternalEvent::Event(Event::Key(KeyEvent::new(
                KeyCode::Char('a'),
                KeyModifiers::SUPER
            )))),
        );
        assert_eq!(
            parse_csi_u_encoded_key_code(b"\x1B[97;17u").unwrap(),
            Some(InternalEvent::Event(Event::Key(KeyEvent::new(
                KeyCode::Char('a'),
                KeyModifiers::HYPER,
            )))),
        );
        assert_eq!(
            parse_csi_u_encoded_key_code(b"\x1B[97;33u").unwrap(),
            Some(InternalEvent::Event(Event::Key(KeyEvent::new(
                KeyCode::Char('a'),
                KeyModifiers::META,
            )))),
        );
    }

    #[test]
    fn test_parse_csi_u_encoded_key_code_with_extra_state() {
        assert_eq!(
            parse_csi_u_encoded_key_code(b"\x1B[97;65u").unwrap(),
            Some(InternalEvent::Event(Event::Key(
                KeyEvent::new_with_kind_and_state(
                    KeyCode::Char('a'),
                    KeyModifiers::empty(),
                    KeyEventKind::Press,
                    KeyEventState::CAPS_LOCK,
                )
            ))),
        );
        assert_eq!(
            parse_csi_u_encoded_key_code(b"\x1B[49;129u").unwrap(),
            Some(InternalEvent::Event(Event::Key(
                KeyEvent::new_with_kind_and_state(
                    KeyCode::Char('1'),
                    KeyModifiers::empty(),
                    KeyEventKind::Press,
                    KeyEventState::NUM_LOCK,
                )
            ))),
        );
    }

    #[test]
    fn test_parse_csi_u_with_shifted_keycode() {
        assert_eq!(
            // A-S-9 is equivalent to A-(
            parse_event(b"\x1B[57:40;4u", false).unwrap(),
            Some(InternalEvent::Event(Event::Key(KeyEvent::new(
                KeyCode::Char('('),
                KeyModifiers::ALT,
            )))),
        );
        assert_eq!(
            // A-S-minus is equivalent to A-_
            parse_event(b"\x1B[45:95;4u", false).unwrap(),
            Some(InternalEvent::Event(Event::Key(KeyEvent::new(
                KeyCode::Char('_'),
                KeyModifiers::ALT,
            )))),
        );
    }

    #[test]
    fn test_parse_csi_special_key_code_with_types() {
        assert_eq!(
            parse_event(b"\x1B[;1:3B", false).unwrap(),
            Some(InternalEvent::Event(Event::Key(KeyEvent::new_with_kind(
                KeyCode::Down,
                KeyModifiers::empty(),
                KeyEventKind::Release,
            )))),
        );
        assert_eq!(
            parse_event(b"\x1B[1;1:3B", false).unwrap(),
            Some(InternalEvent::Event(Event::Key(KeyEvent::new_with_kind(
                KeyCode::Down,
                KeyModifiers::empty(),
                KeyEventKind::Release,
            )))),
        );
    }

    #[test]
    fn test_parse_csi_numbered_escape_code_with_types() {
        assert_eq!(
            parse_event(b"\x1B[5;1:3~", false).unwrap(),
            Some(InternalEvent::Event(Event::Key(KeyEvent::new_with_kind(
                KeyCode::PageUp,
                KeyModifiers::empty(),
                KeyEventKind::Release,
            )))),
        );
        assert_eq!(
            parse_event(b"\x1B[6;5:3~", false).unwrap(),
            Some(InternalEvent::Event(Event::Key(KeyEvent::new_with_kind(
                KeyCode::PageDown,
                KeyModifiers::CONTROL,
                KeyEventKind::Release,
            )))),
        );
    }
}
