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
        // --- 1. TIGHT & PUNCHY LOW (Style Low) ---
        // 歪ませる前に低域をコントロール。Lowが高いほど、重低音の「共鳴」を付加。
        let env = self.envelope[ch];
        let hp_freq = 0.03 + (1.0 - s_low) * 0.2 + (env * 0.15); // タイトさを動的に確保
        self.pre_hp[ch] += hp_freq * (input - self.pre_hp[ch]);
        let mut x = input - self.pre_hp[ch];

        // 低域の「重み」を補強するレゾナンス（Style Lowが高い時のみ）
        let low_boost = s_low * 0.5 * (1.0 - env.min(0.8));
        self.low_resonance[ch] += 0.1 * (x - self.low_resonance[ch]);
        x += self.low_resonance[ch] * low_boost;

        // --- 2. SCOOPED MID (Style Mid) ---
        // Midは「削る」方向にシフト。低いほどモダンなドンシャリ、高いと箱鳴り。
        let scoop = (s_mid - 0.5) * 0.4;
        let mid_cut = x * x * x;
        x -= mid_cut * (0.6 - scoop) ; // 基本的に中域を削って解像度を上げる

        // --- 3. INSANE GAIN STAGE ---
        // ゲイン倍率を大幅に強化 (+100dB級の飽和感)
        let drive = (gain * 15.0).exp() * 10.0;
        x *= drive;

        // --- 4. MULTI-STAGE HARD CLIPPING ---
        // 1段目で深く潰し、2段目で非対称なエッジを立てる
        // Style Highで「クリップの鋭さ」を直接操作
        let hardness = 0.5 + (s_high * 2.0);
        x = (x * hardness).tanh(); // メインの歪み

        // 非対称クリップ（Style Highで高域の食いつきを強化）
        x = if x > 0.0 {
            x.min(0.95)
        } else {
            (x * (1.2 + s_high * 0.5)).atan() * 0.8
        };

        // --- 5. BITE & SHARP EDGE (Style High) ---
        // スルーレート制限を緩和し、高域の「チリチリ」とした攻撃的な成分を開放
        let max_step = 0.05 + (s_high * 0.95);
        let diff = x - self.slew_state[ch];
        self.slew_state[ch] += diff.clamp(-max_step, max_step);

        self.slew_state[ch]
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
