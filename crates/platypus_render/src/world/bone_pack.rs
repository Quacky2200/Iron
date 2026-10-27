// SPDX-License-Identifier: Apache-2.0

//! The bone matrices of the skinned meshes a frame draws, packed into one GPU list: a mesh's run
//! of [`super::WorldView::bones`] is copied in the first time a pass draws it (the shadow maps,
//! a mirror, a view) and every pass after that points at the same copy. Runs no pass draws (a
//! crowd off screen) are never converted nor sent.

use crate::Gpu;
use platypus_core::glam::Mat4;
use std::collections::HashMap;

#[derive(Default)]
pub(crate) struct BonePack {
    /// Where each run (its list, first matrix, count) went in `data`.
    placed: HashMap<(usize, u32, u32), u32>,
    data: Vec<[[f32; 4]; 4]>,
    /// How much of `data` the GPU list holds.
    uploaded: usize,
}

impl BonePack {
    /// Starts a frame's list.
    pub(crate) fn clear(&mut self) {
        self.placed.clear();
        self.data.clear();
        self.uploaded = 0;
    }

    /// Where the run `(start, count)` of `bones` is in the packed list (copied in if new). A run
    /// past the list's end is filled with identities.
    pub(crate) fn place(&mut self, bones: &[Mat4], (start, count): (u32, u32)) -> u32 {
        let data = &mut self.data;
        *self
            .placed
            .entry((bones.as_ptr() as usize, start, count))
            .or_insert_with(|| {
                let at = data.len() as u32;
                let run = bones
                    .get(start as usize..)
                    .unwrap_or(&[])
                    .iter()
                    .take(count as usize);
                data.extend(run.map(Mat4::to_cols_array_2d));
                data.resize(
                    at as usize + count as usize,
                    Mat4::IDENTITY.to_cols_array_2d(),
                );
                at
            })
    }

    /// Sends what was placed since the last call (all of it into a new, larger buffer when
    /// `buffer` is too small: passes already recorded keep the old one).
    pub(crate) fn upload(&mut self, gpu: &Gpu, buffer: &mut wgpu::Buffer) {
        const SIZE: u64 = std::mem::size_of::<[[f32; 4]; 4]>() as u64;
        if buffer.size() < self.data.len() as u64 * SIZE {
            *buffer = super::WorldRenderer::storage::<[[f32; 4]; 4]>(
                gpu,
                "world bones",
                self.data.len().next_power_of_two(),
            );
            self.uploaded = 0;
        }
        if self.uploaded < self.data.len() {
            gpu.queue.write_buffer(
                buffer,
                self.uploaded as u64 * SIZE,
                bytemuck::cast_slice(&self.data[self.uploaded..]),
            );
            self.uploaded = self.data.len();
        }
    }

    /// How many matrices the frame sent so far.
    pub(crate) fn len(&self) -> usize {
        self.data.len()
    }
}
