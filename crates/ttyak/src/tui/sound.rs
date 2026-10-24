//! Notification sounds: short tones synthesized here and played on the speaker chosen in
//! the preferences, or the desktop's own notification sound ("system").

use std::f32::consts::PI;

/// The choices, in the order the preferences cycle through them.
pub const SOUNDS: &[&str] = &["off", "chime", "ping", "knock", "system"];

/// The choices for huddle events (no "system": the desktop has no such sounds).
pub const HUDDLE_SOUNDS: &[&str] = &["off", "rise", "pop", "fall", "chime", "ping", "knock"];

/// The choices for a ringing huddle invitation (played every few seconds).
pub const RING_SOUNDS: &[&str] = &["off", "ring", "chime", "ping", "knock"];

/// The choices for the reminder when you are alone in a huddle.
pub const ALONE_SOUNDS: &[&str] = &["off", "lullaby", "chime"];

/// A soft note: slow attack, long gentle decay (for melodies).
fn soft(hz: f32, ms: u32, volume: f32) -> Vec<f32> {
    let n = (RATE * ms as f32 / 1000.0) as usize;
    (0..n)
        .map(|i| {
            let t = i as f32 / RATE;
            let attack = (i as f32 / (RATE * 0.03)).min(1.0);
            let decay = (-t * 1000.0 / ms as f32 * 2.5).exp();
            ((2.0 * PI * hz * t).sin() + 0.3 * (4.0 * PI * hz * t).sin()) / 1.3 * attack * decay * volume
        })
        .collect()
}

const RATE: f32 = huddle::session::SAMPLE_RATE as f32;

/// A decaying sine burst of `ms` milliseconds.
fn tone(hz: f32, ms: u32, volume: f32) -> Vec<f32> {
    let n = (RATE * ms as f32 / 1000.0) as usize;
    (0..n)
        .map(|i| {
            let t = i as f32 / RATE;
            let attack = (i as f32 / (RATE * 0.004)).min(1.0);
            let decay = (-t * 1000.0 / ms as f32 * 4.0).exp();
            (2.0 * PI * hz * t).sin() * attack * decay * volume
        })
        .collect()
}

fn silence(ms: u32) -> Vec<f32> {
    vec![0.0; (RATE * ms as f32 / 1000.0) as usize]
}

/// 48 kHz mono samples of a built-in sound (empty for `off`, `system` and unknown names).
pub fn samples(name: &str) -> Vec<i16> {
    let f: Vec<f32> = match name {
        "chime" => [tone(880.0, 140, 0.35), tone(1318.5, 220, 0.35)].concat(),
        "ping" => tone(1760.0, 180, 0.3),
        "knock" => [tone(196.0, 60, 0.6), silence(70), tone(196.0, 60, 0.6)].concat(),
        // Joining a huddle: a soft rising C major arpeggio.
        "rise" => [tone(523.25, 90, 0.25), tone(659.25, 90, 0.25), tone(783.99, 200, 0.25)].concat(),
        // Someone joined: a short upward blip; someone left: a falling pair.
        "pop" => [tone(587.33, 60, 0.25), tone(880.0, 120, 0.25)].concat(),
        "fall" => [tone(698.46, 110, 0.25), tone(466.16, 220, 0.25)].concat(),
        // A huddle invitation: a soft two-note trill.
        "ring" => (0..6).flat_map(|i| tone(if i % 2 == 0 { 659.25 } else { 830.61 }, 60, 0.28)).collect(),
        // Alone in a huddle for a while (looped): two slow, quiet pentatonic phrases (~9 s).
        "lullaby" => {
            let phrase = |notes: &[(f32, u32)]| notes.iter().flat_map(|(hz, ms)| soft(*hz, *ms, 0.22)).collect::<Vec<f32>>();
            [
                phrase(&[(523.25, 420), (659.25, 420), (783.99, 420), (880.0, 640), (783.99, 420), (659.25, 420), (523.25, 1000)]),
                silence(500),
                phrase(&[(587.33, 420), (659.25, 420), (783.99, 420), (1046.5, 640), (880.0, 420), (783.99, 420), (659.25, 1100)]),
                silence(900),
            ]
            .concat()
        }
        _ => vec![],
    };
    f.into_iter().map(|s| (s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16).collect()
}

/// Samples as a WAV file: 48 kHz, mono, 16 bit.
#[cfg(all(unix, not(target_os = "macos")))]
fn wav_bytes(samples: &[i16]) -> Vec<u8> {
    let data = samples.len() as u32 * 2;
    let rate = huddle::session::SAMPLE_RATE;
    let mut wav = Vec::with_capacity(44 + data as usize);
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&(36 + data).to_le_bytes());
    wav.extend_from_slice(b"WAVEfmt ");
    wav.extend_from_slice(&16u32.to_le_bytes());
    // PCM, one channel, the rate, bytes per second, bytes per frame, bits per sample.
    wav.extend_from_slice(&1u16.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes());
    wav.extend_from_slice(&rate.to_le_bytes());
    wav.extend_from_slice(&(rate * 2).to_le_bytes());
    wav.extend_from_slice(&2u16.to_le_bytes());
    wav.extend_from_slice(&16u16.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data.to_le_bytes());
    wav.extend(samples.iter().flat_map(|s| s.to_le_bytes()));
    wav
}

/// A built-in sound as a WAV file in the cache folder, written once (None for `off`, `system`, unknown names, or when
/// the cache cannot be written): for a notification server that plays a `sound-file` itself.
#[cfg(all(unix, not(target_os = "macos")))]
pub fn wav_file(name: &str) -> Option<std::path::PathBuf> {
    let samples = samples(name);
    if samples.is_empty() {
        return None;
    }
    let dir = slack_core::session::project_dirs()?.cache_dir().join("sounds");
    let path = dir.join(format!("{name}.wav"));
    let wav = wav_bytes(&samples);
    if std::fs::metadata(&path).is_ok_and(|m| m.len() == wav.len() as u64) {
        return Some(path);
    }
    std::fs::create_dir_all(&dir).ok()?;
    // Written whole beside it, then renamed: a server never reads half a sound.
    static PARTS: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let part = dir.join(format!("{name}.wav.{}.{}", std::process::id(), PARTS.fetch_add(1, std::sync::atomic::Ordering::Relaxed)));
    std::fs::write(&part, &wav).ok()?;
    std::fs::rename(&part, &path).ok()?;
    Some(path)
}

/// Play a built-in sound without blocking (no-op for `off`/`system`). `device`: part of an
/// output device's name, None for the default; `volume` in percent.
pub fn play(name: &str, device: Option<String>, volume: u32) {
    let samples = samples(name);
    if samples.is_empty() {
        return;
    }
    std::thread::spawn(move || match huddle::devices::Speaker::start(device) {
        Ok(mut speaker) => {
            speaker.volume().set(volume as f32 / 100.0);
            // The speaker queues at most 0.5 s: feed longer sounds in 100 ms parts, two ahead.
            let part = RATE as usize / 10;
            for (i, chunk) in samples.chunks(part).enumerate() {
                speaker.play(chunk);
                if i >= 2 {
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }
            }
            // Let it drain before the stream is dropped.
            std::thread::sleep(std::time::Duration::from_millis(450));
        }
        Err(e) => tracing::warn!("notification sound: {e}"),
    });
}

/// A sound playing over and over until this is dropped.
pub struct Looping(std::sync::Arc<std::sync::atomic::AtomicBool>);

impl Drop for Looping {
    fn drop(&mut self) {
        self.0.store(true, std::sync::atomic::Ordering::Relaxed);
    }
}

/// Play a built-in sound in a loop (None for `off`/`system`), fed like [`play`].
pub fn play_looping(name: &str, device: Option<String>, volume: u32) -> Option<Looping> {
    let samples = samples(name);
    if samples.is_empty() {
        return None;
    }
    let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let stopped = stop.clone();
    std::thread::spawn(move || match huddle::devices::Speaker::start(device) {
        Ok(mut speaker) => {
            speaker.volume().set(volume as f32 / 100.0);
            let part = RATE as usize / 10;
            let mut ahead = 0;
            'outer: loop {
                for chunk in samples.chunks(part) {
                    if stopped.load(std::sync::atomic::Ordering::Relaxed) {
                        break 'outer;
                    }
                    speaker.play(chunk);
                    ahead += 1;
                    if ahead > 2 {
                        std::thread::sleep(std::time::Duration::from_millis(100));
                    }
                }
            }
        }
        Err(e) => tracing::warn!("looping sound: {e}"),
    });
    Some(Looping(stop))
}

/// The notification server's sound name for "system".
pub fn system_sound_name() -> &'static str {
    if cfg!(target_os = "macos") {
        "NSUserNotificationDefaultSoundName"
    } else if cfg!(target_os = "windows") {
        "Default"
    } else {
        "message-new-instant"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn built_in_sounds() {
        assert!(samples("off").is_empty() && samples("system").is_empty() && samples("nope").is_empty());
        let chime = samples("chime");
        assert_eq!(chime.len(), (RATE * 0.36) as usize, "two tones, 360 ms");
        assert!(chime.iter().map(|s| s.unsigned_abs()).max().unwrap() > 5_000, "audible");
        assert!(chime.len() < huddle::session::SAMPLE_RATE as usize / 2, "fits the speaker's 0.5 s queue");
        assert!(samples("knock").len() < samples("chime").len());
        for name in HUDDLE_SOUNDS.iter().chain(RING_SOUNDS).filter(|n| **n != "off") {
            let s = samples(name);
            assert!(!s.is_empty() && s.len() < huddle::session::SAMPLE_RATE as usize / 2, "{name}: audible and short");
        }
        let lullaby = samples("lullaby");
        let secs = lullaby.len() as f32 / RATE;
        let peak = lullaby.iter().map(|s| s.unsigned_abs()).max().unwrap();
        assert!((8.0..11.0).contains(&secs) && (2_000..9_000).contains(&peak), "two quiet phrases of ~9 s: {secs} s, peak {peak}");
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    #[test]
    fn a_sound_as_a_wav_file() {
        let path = wav_file("chime").expect("the cache folder is writable");
        let wav = std::fs::read(&path).unwrap();
        assert_eq!((&wav[..4], &wav[8..16]), (&b"RIFF"[..], &b"WAVEfmt "[..]));
        assert_eq!(wav.len(), 44 + samples("chime").len() * 2);
        assert_eq!(wav_file("chime"), Some(path), "written once, found again");
        assert!(wav_file("off").is_none() && wav_file("system").is_none() && wav_file("nope").is_none());
    }
}
