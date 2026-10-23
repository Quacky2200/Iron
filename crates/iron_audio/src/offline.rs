// SPDX-License-Identifier: Apache-2.0

use std::sync::{Arc, Mutex};

use kira::backend::{Backend, Renderer};

/// A backend with no device: the mix is rendered on demand with [`Self::render`].
/// For tests, and for running games where no output device exists.
pub struct OfflineBackend {
    sample_rate: u32,
    renderer: Arc<Mutex<Option<Renderer>>>,
}

impl OfflineBackend {
    /// Mixes the next `frames` stereo frames and returns them.
    pub fn render(&mut self, frames: usize) -> Vec<[f32; 2]> {
        let mut out = vec![0.0; frames * 2];
        if let Ok(mut guard) = self.renderer.lock()
            && let Some(renderer) = guard.as_mut()
        {
            renderer.on_start_processing();
            renderer.process(&mut out, 2);
        }
        out.as_chunks::<2>().0.to_vec()
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }
}

impl Backend for OfflineBackend {
    /// The sample rate to mix at.
    type Settings = u32;
    type Error = ();

    fn setup(sample_rate: u32, _internal_buffer_size: usize) -> Result<(Self, u32), ()> {
        Ok((
            Self {
                sample_rate,
                renderer: Arc::default(),
            },
            sample_rate,
        ))
    }

    fn start(&mut self, renderer: Renderer) -> Result<(), ()> {
        *self.renderer.lock().map_err(|_| ())? = Some(renderer);
        Ok(())
    }
}
