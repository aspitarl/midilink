use std::{sync::mpsc::Receiver, time::{Duration, Instant}};

use anyhow::{Context, Result};
use midly::{live::LiveEvent, MidiMessage};
use rusty_link::{AblLink, SessionState};
use tracing::{debug, info, trace};

pub struct LinkAdapter {
    link: AblLink,
    state: SessionState,
    /// The number of beats per bar/phrase in the Link session.
    quantum: u32,
    /// The position of the current beat within the quantum.
    beat_in_quantum: u32,
    last_beat_at: Option<Instant>,
    /// Receives the times at which the user marked a downbeat.
    downbeats: Receiver<Instant>,
}

impl LinkAdapter {
    /// Creates a new adapter that automatically connects to Ableton Link.
    pub fn new(quantum: u32, downbeats: Receiver<Instant>) -> Self {
        let link = AblLink::new(120.0);
        let state = SessionState::new();

        info!("Enabling Link");
        link.enable(true);

        link.set_num_peers_callback(move |peers| {
            info!(peers, "Link updated");
        });

        Self {
            link,
            state,
            quantum: quantum.max(1),
            beat_in_quantum: 0,
            last_beat_at: None,
            downbeats,
        }
    }

    /// Advances the position within the quantum for a beat arriving now.
    /// Playback restarts (i.e. a long gap since the last beat) and explicit
    /// downbeat marks reset the position.
    fn advance_beat(&mut self) -> u32 {
        let now = Instant::now();
        let beat_len = Duration::from_secs_f64(60.0 / self.tempo().max(1.0));
        let restarted = self.last_beat_at
            .map_or(true, |last| now.duration_since(last) > beat_len.mul_f64(1.5));

        self.beat_in_quantum = if restarted {
            0
        } else {
            (self.beat_in_quantum + 1) % self.quantum
        };

        // Users tend to mark downbeats slightly late, so a mark shortly after
        // the previous beat refers to that beat rather than the current one.
        if let Some(marked_at) = self.downbeats.try_iter().last() {
            let marked_previous = self.last_beat_at.is_some_and(|last| {
                marked_at.saturating_duration_since(last) < beat_len / 2
            });
            self.beat_in_quantum = if marked_previous { 1 % self.quantum } else { 0 };
        }

        self.last_beat_at = Some(now);
        self.beat_in_quantum
    }

    fn tempo(&mut self) -> f64 {
        self.link.capture_audio_session_state(&mut self.state);
        self.state.tempo()
    }

    /// Handles an incoming MIDI event in raw form.
    pub fn handle_raw_event(&mut self, stamp: u64, raw: &[u8]) -> Result<()> {
        let event = LiveEvent::parse(raw).context("Could not parse MIDI event")?;
        self.handle_event(stamp, event)
    }

    /// Handles an incoming MIDI event.
    pub fn handle_event(&mut self, stamp: u64, event: LiveEvent) -> Result<()> {
        trace!(?event, "Handling");
        match event {
            LiveEvent::Midi { channel: _, message: MidiMessage::NoteOn { key, vel } } => {
                // See https://github.com/mixxxdj/mixxx/wiki/MIDI%20clock%20output
                match key.as_int() {
                    // Beat
                    0x32 if vel == 100 => {
                        // Mixxx does not expose a concept of bars, therefore
                        // we count beats ourselves and let the user mark the
                        // downbeat (or assume one after playback restarts).
                        // See https://ableton.github.io/link for details.
                        // We 'rudely' force the beat to unidirectionally bridge
                        // the Mixxx clock into the Link session. This also
                        // implies that only a single MIDIlink instance should
                        // run per session.
                        let beat = self.advance_beat() as f64;
                        let quantum = self.quantum as f64;
                        // The MIDI driver's timestamp is not on the Link clock,
                        // so we use the time at which the event arrived.
                        let time = self.link.clock_micros();
                        info!(stamp, time, beat, "Setting beat");
                        self.update_state(|state| state.force_beat_at_time(beat, time as u64, quantum));
                    },
                    // BPM
                    0x34 => {
                        let bpm = vel.as_int() + 50;
                        let time = self.link.clock_micros();
                        info!(bpm, "Setting BPM");
                        self.update_state(|state| state.set_tempo(bpm as f64, time));
                    },
                    _ => debug!(?event, "Ignoring MIDI note event"),
                }
            },
            _ => debug!(?event, "Ignoring MIDI event"),
        }

        Ok(())
    }

    fn update_state(&mut self, action: impl FnOnce(&mut SessionState)) {
        self.link.capture_audio_session_state(&mut self.state);
        action(&mut self.state);
        self.link.commit_audio_session_state(&mut self.state);
    }
}

impl Drop for LinkAdapter {
    fn drop(&mut self) {
        info!("Disabling Link");
        self.link.enable(false);
    }
}
