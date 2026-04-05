use crate::params::MetalXrossParams;
use crate::xross::gain::XrossGainProcessor;
use nih_plug::prelude::*;
use std::sync::Arc;

pub struct XrossMetalSystem {
    params: Arc<MetalXrossParams>,
    pre_hp: [f32; 2],
    pre_res: [f32; 2],
    slew_state: [f32; 2],
    dc_block: [f32; 2],
    envelope: [f32; 2],
    prev_input: [f32; 2],
    os_lpf: [f32; 2],
    os_lpf_2: [f32; 2],
    low_resonance: [f32; 2],
}

impl XrossMetalSystem {
    pub fn new(params: Arc<MetalXrossParams>) -> Self {
        Self {
            params,
            pre_hp: [0.0; 2],
            pre_res: [0.0; 2],
            slew_state: [0.0; 2],
            dc_block: [0.0; 2],
            envelope: [0.0; 2],
            prev_input: [0.0; 2],
            os_lpf: [0.0; 2],
            os_lpf_2: [0.0; 2],
            low_resonance: [0.0; 2],
        }
    }

    #[inline]
    fn drive_core(&mut self, input: f32, env: f32, ch: usize, os_inv: f32) -> f32 {
        let gain = self.params.general.gain.value();
        let s_low = self.params.style.low.value();
        let s_mid = self.params.style.mid.value();
        let s_high = self.params.style.high.value();

        let hp_freq = (0.155 + (1.0 - s_low) * 0.115) * os_inv;
        self.pre_hp[ch] += hp_freq * (input - self.pre_hp[ch]);
        let mut x = input - self.pre_hp[ch];

        let res_freq = 0.265 * os_inv;
        let res_q = 0.52 + (s_mid * 0.48);
        self.pre_res[ch] += res_freq * (x - self.pre_res[ch]);
        x = x + (x - self.pre_res[ch]) * (2.8 * res_q);
        self.pre_res[ch] *= 0.98;

        let drive = 5.0 + (gain * 30.0);
        x *= drive;

        x = if x > 0.0 {
            (x * 2.5).atan() * 0.9
        } else {
            (x * 2.0).tanh() * 1.1
        };

        let hard_warp = 2.0 + (gain * 30.0);
        x = (x * hard_warp).clamp(-0.90, 0.90);

        let scoop_depth = (0.7 - s_mid).max(0.0) * 1.6;
        let scoop_filter = x.powi(3) - x * 0.3;
        x -= scoop_filter * scoop_depth;

        let bite_base = 0.12 + (s_high * 0.55);
        let bite = bite_base * (env * 0.85 + 0.15).min(1.0);
        let diff = x - self.slew_state[ch];
        self.slew_state[ch] += diff.clamp(-bite, bite);
        x = self.slew_state[ch];

        let punch_amount = s_low * 1.35;
        let punch_freq = 0.08 * os_inv;
        self.low_resonance[ch] += punch_freq * (x - self.low_resonance[ch]);
        x += self.low_resonance[ch] * punch_amount;

        x.clamp(-1.0, 1.0)
    }

    fn process_sample(&mut self, input: f32, ch: usize) -> f32 {
        let os_factor = 4;
        let inv_os = 1.0 / os_factor as f32;
        let target = input.abs();

        let env_step = if target > self.envelope[ch] {
            0.3
        } else {
            0.01
        };
        self.envelope[ch] += env_step * (target - self.envelope[ch]);

        let mut output_sum = 0.0;
        for i in 0..os_factor {
            let fraction = i as f32 * inv_os;
            let sub_sample = self.prev_input[ch] + (input - self.prev_input[ch]) * fraction;
            output_sum += self.drive_core(sub_sample, self.envelope[ch], ch, inv_os);
        }

        self.prev_input[ch] = input;
        let raw_out = output_sum * inv_os;

        let lpf_cutoff = 0.48;
        self.os_lpf[ch] += lpf_cutoff * (raw_out - self.os_lpf[ch]);
        self.os_lpf_2[ch] += lpf_cutoff * (self.os_lpf[ch] - self.os_lpf_2[ch]);

        let out = self.os_lpf_2[ch];

        let dc_fix = out - self.dc_block[ch];
        self.dc_block[ch] = out + 0.995 * (self.dc_block[ch] - out);

        dc_fix * 0.82
    }
}

impl XrossGainProcessor for XrossMetalSystem {
    fn initialize(
        &mut self,
        _layout: &AudioIOLayout,
        _config: &BufferConfig,
        _context: &mut impl InitContext<crate::MetalXross>,
    ) -> bool {
        self.pre_hp = [0.0; 2];
        self.pre_res = [0.0; 2];
        self.slew_state = [0.0; 2];
        self.dc_block = [0.0; 2];
        self.envelope = [0.0; 2];
        self.prev_input = [0.0; 2];
        self.os_lpf = [0.0; 2];
        self.os_lpf_2 = [0.0; 2];
        self.low_resonance = [0.0; 2];
        true
    }

    fn process_channel(&mut self, slice: &mut [f32], ch_idx: usize) {
        let ch = ch_idx % 2;
        for sample in slice {
            *sample = self.process_sample(*sample, ch);
        }
    }
}
