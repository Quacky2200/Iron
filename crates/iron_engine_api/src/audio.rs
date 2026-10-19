//! `audio`: load a sound, play it, pause and resume the playback. This slice records what scripts
//! ask for as [`AudioCommand`]s; the host takes them with [`take_commands`] and drives the mixer
//! (`iron_audio`). Options are trailing parameters: `audio.load(path, { stream: true })` and
//! `sound.play({ loop: true })` pass `stream` and `loop` (left out: false).

use iron_api::{Handle, iron_api};
use std::cell::RefCell;

/// A loaded sound (1-based; 0 is "no sound").
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SoundRef(pub u32);

/// A playing (or paused) sound (1-based).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlaybackRef(pub u32);

impl Handle for SoundRef {
    fn to_raw(self) -> u64 {
        u64::from(self.0)
    }

    fn from_raw(raw: u64) -> Self {
        SoundRef(raw as u32)
    }
}

impl Handle for PlaybackRef {
    fn to_raw(self) -> u64 {
        u64::from(self.0)
    }

    fn from_raw(raw: u64) -> Self {
        PlaybackRef(raw as u32)
    }
}

/// What a script asked the audio system to do.
#[derive(Clone, Debug, PartialEq)]
pub enum AudioCommand {
    Load {
        sound: SoundRef,
        path: String,
        stream: bool,
    },
    Play {
        sound: SoundRef,
        playback: PlaybackRef,
        looped: bool,
    },
    Pause(PlaybackRef),
    Resume(PlaybackRef),
    Stop(PlaybackRef),
}

#[derive(Default)]
struct State {
    sounds: Vec<String>,
    playbacks: Vec<(SoundRef, bool)>,
    commands: Vec<AudioCommand>,
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State::default());
}

/// The commands since the last call, in order.
pub fn take_commands() -> Vec<AudioCommand> {
    STATE.with(|s| std::mem::take(&mut s.borrow_mut().commands))
}

pub fn reset() {
    STATE.with(|s| *s.borrow_mut() = State::default());
}

#[iron_api(namespace = "audio", handles(Sound = SoundRef, Playback = PlaybackRef))]
pub mod audio_api {
    use super::{AudioCommand, PlaybackRef, STATE, SoundRef};

    /// Loads a sound from a path inside the game folder.
    pub fn load(path: &str, stream: bool) -> SoundRef {
        STATE.with(|s| {
            let mut s = s.borrow_mut();
            s.sounds.push(path.to_string());
            let sound = SoundRef(s.sounds.len() as u32);
            s.commands.push(AudioCommand::Load {
                sound,
                path: path.to_string(),
                stream,
            });
            sound
        })
    }

    impl SoundRef {
        /// Starts the sound and returns the playback.
        pub fn play(self, r#loop: bool) -> PlaybackRef {
            STATE.with(|s| {
                let mut s = s.borrow_mut();
                s.playbacks.push((self, false));
                let playback = PlaybackRef(s.playbacks.len() as u32);
                s.commands.push(AudioCommand::Play {
                    sound: self,
                    playback,
                    looped: r#loop,
                });
                playback
            })
        }

        #[property]
        pub fn path(self) -> String {
            STATE.with(|s| {
                s.borrow()
                    .sounds
                    .get((self.0 as usize).wrapping_sub(1))
                    .cloned()
                    .unwrap_or_default()
            })
        }
    }

    impl PlaybackRef {
        pub fn pause(self) {
            STATE.with(|s| {
                let mut s = s.borrow_mut();
                if let Some(p) = s.playbacks.get_mut((self.0 as usize).wrapping_sub(1)) {
                    p.1 = true;
                    s.commands.push(AudioCommand::Pause(self));
                }
            });
        }

        pub fn resume(self) {
            STATE.with(|s| {
                let mut s = s.borrow_mut();
                if let Some(p) = s.playbacks.get_mut((self.0 as usize).wrapping_sub(1)) {
                    p.1 = false;
                    s.commands.push(AudioCommand::Resume(self));
                }
            });
        }

        pub fn stop(self) {
            STATE.with(|s| s.borrow_mut().commands.push(AudioCommand::Stop(self)));
        }

        #[property]
        pub fn paused(self) -> bool {
            STATE.with(|s| {
                s.borrow()
                    .playbacks
                    .get((self.0 as usize).wrapping_sub(1))
                    .is_some_and(|p| p.1)
            })
        }
    }
}
