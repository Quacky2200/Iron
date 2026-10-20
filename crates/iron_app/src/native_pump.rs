//! What compiled games hand to the app each frame, apart from drawing: audio commands for the
//! mixer, and the finished sounds going back (`iron_engine_api::audio`).

use super::App;
use iron_core::Value;
use iron_engine_api::audio::{AudioCommand, PlaybackRef};

impl App {
    /// Applies the audio commands compiled code queued to the mixer. The mixer opens on the first
    /// command, as for QuickJS games.
    pub(super) fn native_audio_commands(&mut self) {
        if self.native.is_none() {
            return;
        }
        for c in iron_engine_api::audio::take_commands() {
            match &c {
                AudioCommand::Load {
                    sound,
                    path,
                    stream,
                } => self.audio_op(
                    "load",
                    &[
                        Value::Number(f64::from(sound.0)),
                        Value::String(path.clone()),
                        Value::Bool(*stream),
                    ],
                ),
                other => {
                    if let Some((name, args)) = other.to_op() {
                        let args: Vec<Value> = args.iter().map(core_value).collect();
                        self.audio_op(name, &args);
                    }
                }
            }
        }
    }

    /// Sounds the mixer finished go back to the program (`onEnded`, `AudioSource` state).
    pub(super) fn native_audio_ended(&mut self, ended: &[u32]) {
        let ids: Vec<PlaybackRef> = ended.iter().map(|i| PlaybackRef(*i)).collect();
        if let Some(n) = self.native.as_mut() {
            n.audio_ended(&mut self.scene.scene, &mut self.ui, &ids);
        }
    }
}

/// The engine API's JSON values as the values ops carry.
fn core_value(v: &serde_json::Value) -> Value {
    match v {
        serde_json::Value::Null => Value::Null,
        serde_json::Value::Bool(b) => Value::Bool(*b),
        serde_json::Value::Number(n) => Value::Number(n.as_f64().unwrap_or(0.0)),
        serde_json::Value::String(s) => Value::String(s.clone()),
        serde_json::Value::Array(a) => Value::Array(a.iter().map(core_value).collect()),
        serde_json::Value::Object(_) => Value::Null,
    }
}
