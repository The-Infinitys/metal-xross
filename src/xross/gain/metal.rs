use crate::params::MetalXrossParams;
use crate::xross::gain::XrossGainProcessor;
use nih_plug::prelude::*;
use std::sync::Arc;

pub struct XrossMetalSystem {
    params: Arc<MetalXrossParams>,
    pre_hp: [f32; 2],
    slew_state: [f32; 2],
    dc_block: [f32; 2],
    envelope: [f32; 2],
    prev_input: [f32; 2],
    os_lpf: [f32; 2],
    // 低域の迫力を出すための共鳴フィルタ
    low_resonance: [f32; 2],
}

impl XrossMetalSystem {
    pub fn new(params: Arc<MetalXrossParams>) -> Self {
        Self {
            params,
            pre_hp: [0.0; 2],
            slew_state: [0.0; 2],
            dc_block: [0.0; 2],
            envelope: [0.0; 2],
            prev_input: [0.0; 2],
            os_lpf: [0.0; 2],
            low_resonance: [0.0; 2],
        }
    }
    #[inline]
    fn drive_core(
        &mut self,
        input: f32,
        gain: f32,
        s_low: f32,
        s_mid: f32,
        s_high: f32,
        ch: usize,
    ) -> f32 {
        // 1. 適度なプリ・ゲイン (「棒」にならない程度に抑える)
        // gain 0.5付近で標準的なメタルサウンドになるよう調整
        let pre_gain = 5.0 + (gain * 55.0);
        let mut x = input * pre_gain;

        // 2. Pre-EQ (MT-2特有の削り)
        let hp_freq = 0.1 + (1.0 - s_low) * 0.15;
        self.pre_hp[ch] += hp_freq * (x - self.pre_hp[ch]);
        x -= self.pre_hp[ch];

        // 3. Multi-Stage Soft Clipping (ここが重要)
        // 一気に潰さず、段階的に。

        // Stage 1: 非対称な飽和
        x = if x > 0.0 { x.atan() } else { (x * 0.9).tanh() };

        // Stage 2: ゲイン調整
        // ここで倍率を上げすぎると「潰れた棒」になります。
        x *= 2.0 + (gain * 10.0);

        // Stage 3: Soft Knee (最終的な形を整える)
        // clampの代わりにこれを使うことで、波形の頂点に「丸み」を与えます。
        let soft_limit = 0.95;
        if x.abs() > soft_limit {
            x = soft_limit * (x / soft_limit).atan();
        }

        // 4. MT-2 Active EQ
        // Mid Scoop: 削りすぎるとスカスカになるので、s_mid 0.5を基準に。
        let scoop = (0.5 - s_mid).max(0.0) * 0.8;
        x -= (x * x * x) * scoop;

        // High: 金属的な質感を出すスルーレート
        let bite = 0.15 + (s_high * 0.7);
        let diff = x - self.slew_state[ch];
        self.slew_state[ch] += diff.clamp(-bite, bite);
        x = self.slew_state[ch];

        // 5. Low / Punch (歪んだ後の厚み)
        let punch = s_low * 0.4;
        self.low_resonance[ch] += 0.1 * (x - self.low_resonance[ch]);
        x += self.low_resonance[ch] * punch;

        // 最終リミッター (安全策)
        x.clamp(-1.0, 1.0)
    }
    fn process_sample(
        &mut self,
        input: f32,
        gain: f32,
        s_low: f32,
        s_mid: f32,
        s_high: f32,
        ch: usize,
    ) -> f32 {
        // メタルは常に高密度な歪みが必要なため、OS倍率の閾値を下げる
        let os_factor = if gain < 0.2 {
            1
        } else if gain < 0.5 {
            2
        } else {
            4
        };

        // 非常に速いアタックへの追従
        self.envelope[ch] += (input.abs() - self.envelope[ch]) * 0.1;

        let mut output_sum = 0.0;
        let inv_os = 1.0 / os_factor as f32;

        for i in 0..os_factor {
            let fraction = i as f32 * inv_os;
            let sub_sample = self.prev_input[ch] + (input - self.prev_input[ch]) * fraction;
            output_sum += self.drive_core(sub_sample, gain, s_low, s_mid, s_high, ch);
        }
        self.prev_input[ch] = input;

        // OS LPF (メタル用に少し高域を残す調整: 0.5 -> 0.7)
        let raw_out = output_sum * inv_os;
        self.os_lpf[ch] += 0.7 * (raw_out - self.os_lpf[ch]);

        let out = self.os_lpf[ch];
        let dc_fix = out - self.dc_block[ch];
        self.dc_block[ch] = out + 0.998 * (self.dc_block[ch] - out);

        dc_fix * 0.6
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
        self.slew_state = [0.0; 2];
        self.dc_block = [0.0; 2];
        self.envelope = [0.0; 2];
        self.prev_input = [0.0; 2];
        self.os_lpf = [0.0; 2];
        self.low_resonance = [0.0; 2];
        true
    }

    fn process_channel(&mut self, slice: &mut [f32], ch_idx: usize) {
        let g = self.params.general.gain.value();
        let sl = self.params.style.low.value();
        let sm = self.params.style.mid.value();
        let sh = self.params.style.high.value();
        let ch = ch_idx % 2;

        for sample in slice {
            *sample = self.process_sample(*sample, g, sl, sm, sh, ch);
        }
    }
}
