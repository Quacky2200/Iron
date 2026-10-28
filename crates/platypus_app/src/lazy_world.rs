// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

//! The 3D world renderer, built on first use on the light path's wgpu fallback.
//!
//! `WorldRenderer::new` compiles the world shaders and pipelines, which costs the GPU driver
//! tens of MB even when a game is only UI. A UI-only game never has a camera or a mesh upload,
//! so it never asks for the renderer. When the host chose the world (`host::Choice::World`) the
//! renderer is built at once.

use platypus_render::{Gpu, WorldRenderer};

pub(crate) struct LazyWorld {
    inner: Option<WorldRenderer>,
    physical_units: bool,
}

impl LazyWorld {
    pub(crate) fn new(gpu: &Gpu, physical_units: bool) -> Self {
        let mut world = Self {
            inner: None,
            physical_units,
        };
        if !super::host::ui_only() {
            world.get(gpu);
        }
        world
    }

    /// The renderer, built now if it wasn't yet.
    pub(crate) fn get(&mut self, gpu: &Gpu) -> &mut WorldRenderer {
        let physical_units = self.physical_units;
        self.inner.get_or_insert_with(|| {
            let mut world = WorldRenderer::new(gpu);
            world.physical_units = physical_units;
            if physical_units {
                tracing::info!("real-world light units");
            }
            world
        })
    }

    /// The renderer when `needed` or when it exists already; `None` while it's not wanted.
    pub(crate) fn ensure(&mut self, gpu: &Gpu, needed: bool) -> Option<&mut WorldRenderer> {
        (needed || self.inner.is_some()).then(|| self.get(gpu))
    }

    /// The renderer if it exists.
    pub(crate) fn built(&mut self) -> Option<&mut WorldRenderer> {
        self.inner.as_mut()
    }
}
