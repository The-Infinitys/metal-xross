use crate::{MetalXross, params::MetalXrossParams};
use nih_plug::prelude::*;
use std::sync::Arc;
// NonZeroU32のインポートは不要になったので削除

mod crunch;
mod dist;
mod drive;
mod metal;

use crunch::XrossCrunchSystem;
use dist::XrossDistSystem;
use drive::XrossDriveSystem;
use metal::XrossMetalSystem;

pub trait XrossGainProcessor {
    fn initialize(
        &mut self,
        layout: &AudioIOLayout,
        config: &BufferConfig,
        context: &mut impl InitContext<MetalXross>,
    ) -> bool;
    fn process(
        &mut self,
        buffer: &mut Buffer,
        aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<MetalXross>,
    );
}

pub struct XrossGainSystem {
    params: Arc<MetalXrossParams>,
    crunch: XrossCrunchSystem,
    drive: XrossDriveSystem,
    dist: XrossDistSystem,
    metal: XrossMetalSystem,
    tmp_buffer_data: Vec<f32>,
    buffer_size: usize,
    num_channels: usize,
}

impl XrossGainSystem {
    pub fn new(params: Arc<MetalXrossParams>) -> Self {
        Self {
            params: params.clone(),
            crunch: XrossCrunchSystem::new(params.clone()),
            drive: XrossDriveSystem::new(params.clone()),
            dist: XrossDistSystem::new(params.clone()),
            metal: XrossMetalSystem::new(params.clone()),
            tmp_buffer_data: Vec::new(),
            buffer_size: 0,
            num_channels: 0,
        }
    }

    pub fn initialize(
        &mut self,
        audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        context: &mut impl InitContext<MetalXross>,
    ) -> bool {
        self.num_channels = audio_io_layout
            .main_output_channels
            .map(|n| n.get())
            .unwrap_or(2) as usize;

        self.buffer_size = buffer_config.max_buffer_size as usize;
        self.tmp_buffer_data = vec![0.0; self.num_channels * self.buffer_size];

        self.crunch
            .initialize(audio_io_layout, buffer_config, context);
        self.drive
            .initialize(audio_io_layout, buffer_config, context);
        self.dist
            .initialize(audio_io_layout, buffer_config, context);
        self.metal
            .initialize(audio_io_layout, buffer_config, context);

        true
    }

    pub fn process(
        &mut self,
        buffer: &mut Buffer,
        aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<MetalXross>,
    ) {
        let style = self.params.style.value();
        let idx_a = (style.floor() as usize).min(3);
        let idx_b = (idx_a + 1).min(3);
        let fraction = style - idx_a as f32;

        if fraction <= 0.001 || idx_a == idx_b {
            self.dispatch_process(idx_a, buffer, aux, context);
            return;
        }

        let num_samples = buffer.samples();

        for (ch, slice) in buffer.as_slice().iter().enumerate() {
            if ch < self.num_channels {
                let dest_start = ch * self.buffer_size;
                self.tmp_buffer_data[dest_start..dest_start + num_samples].copy_from_slice(slice);
            }
        }

        self.dispatch_process(idx_a, buffer, aux, context);

        let mut b_slices: Vec<&mut [f32]> = (0..self.num_channels)
            .map(|ch| {
                let start = ch * self.buffer_size;
                unsafe {
                    std::slice::from_raw_parts_mut(
                        self.tmp_buffer_data.as_mut_ptr().add(start),
                        num_samples,
                    )
                }
            })
            .collect();

        // create_buffer_from_slices 呼び出し
        let mut buffer_b = unsafe { self.create_buffer_from_slices(&mut b_slices, num_samples) };
        self.dispatch_process(idx_b, &mut buffer_b, aux, context);

        for (ch, slice_a) in buffer.as_slice().iter_mut().enumerate() {
            if ch < self.num_channels {
                let offset_b = ch * self.buffer_size;
                let slice_b = &self.tmp_buffer_data[offset_b..offset_b + num_samples];

                for (s_a, &s_b) in slice_a.iter_mut().zip(slice_b.iter()) {
                    *s_a = *s_a * (1.0 - fraction) + s_b * fraction;
                }
            }
        }
    }

    fn dispatch_process(
        &mut self,
        index: usize,
        buffer: &mut Buffer,
        aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<MetalXross>,
    ) {
        match index {
            0 => self.crunch.process(buffer, aux, context),
            1 => self.drive.process(buffer, aux, context),
            2 => self.dist.process(buffer, aux, context),
            _ => self.metal.process(buffer, aux, context),
        }
    }

    // unsafe 操作を明示的に block で囲む
    unsafe fn create_buffer_from_slices<'a>(
        &self,
        slices: &mut Vec<&'a mut [f32]>,
        samples: usize,
    ) -> Buffer<'a> {
        let slices_ptr = slices as *mut Vec<&'a mut [f32]> as *mut Vec<&'static mut [f32]>;
        let data = (samples, unsafe { std::ptr::read(slices_ptr) });
        unsafe { std::mem::transmute::<_, Buffer<'a>>(data) }
    }
}
