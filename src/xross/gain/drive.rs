use crate::MetalXross;
use crate::params::MetalXrossParams;
use crate::xross::gain::XrossGainProcessor;
use nih_plug::prelude::*;
use std::sync::Arc;

pub struct XrossDriveSystem {
    params: Arc<MetalXrossParams>,
    low_cut: Vec<f32>,
    mid_shaper: Vec<f32>,
    lpf_state: Vec<f32>,
    prev_input: Vec<f32>,
    // 内部OS用の状態保持
    os_prev_sub: Vec<f32>,
}

impl XrossDriveSystem {
    pub fn new(params: Arc<MetalXrossParams>) -> Self {
        Self {
            params,
            low_cut: vec![0.0; 2],
            mid_shaper: vec![0.0; 2],
            lpf_state: vec![0.0; 2],
            prev_input: vec![0.0; 2],
            os_prev_sub: vec![0.0; 2],
        }
    }

    /// よりダイナミックレンジの広いチューブ・サチュレーション
    #[inline(always)]
    fn modern_drive_shaper(&self, x: f32, bite: f32) -> f32 {
        let x = x * (1.0 + bite); // 高域の「噛みつき」成分をブースト
        // ソフトクリップだが、一定を超えると急激に飽和するハイブリッド特性
        if x.abs() < 0.3 {
            x
        } else {
            x.tanh() // 伝統的なODの質感を出すためにtanhを採用
        }
    }

    fn drive_core(
        &mut self,
        input: f32,
        gain: f32,
        s_low: f32,
        s_mid: f32,
        s_high: f32,
        ch: usize,
    ) -> f32 {
        // 1. DYNAMIC EDGE (ピッキングへの反応性)
        // 微分成分を取り出し、アタックの瞬間だけドライブを深くする
        let diff = input - self.prev_input[ch];
        self.prev_input[ch] = input;
        let attack_boost = diff * (1.0 + gain * 2.0);
        let shaped_input = input + attack_boost * 0.3;

        // 2. MODERN TIGHTNESS (Style Low)
        // 500Hz付近をStyle Lowに応じてカットし、モダンなタイトさを出す
        let hp_freq = 0.04 + (1.0 - s_low).powf(1.5) * 0.15;
        let filtered = shaped_input - self.low_cut[ch];
        self.low_cut[ch] = shaped_input * hp_freq + self.low_cut[ch] * (1.0 - hp_freq);

        // 3. SEPARATION PHASE (Style Mid)
        // コードの分離感を出すための位相操作。Style Midが高いほど「前」に出る。
        let p_coeff = 0.2 + s_mid * 0.4;
        let phased = p_coeff * filtered + self.mid_shaper[ch];
        self.mid_shaper[ch] = filtered - p_coeff * phased;

        // 4. POWER DRIVE STAGE
        // ゲインを大幅強化。0.0でもクランチ、1.0でモダンロックなリードまでカバー。
        let drive_amount = (gain * 4.5).exp() * 12.0;
        let mut x = phased * drive_amount;

        // Stage 1: 非対称性を加え、ピッキングの「食いつき」を作る
        let bite = s_high * 0.5;
        x = self.modern_drive_shaper(x + 0.05, bite);

        // Stage 2: 中域の粘り (Style Mid)
        let mid_gain = 1.0 + s_mid * 1.5;
        x = (x * mid_gain).tanh();

        // 5. TONE SHAPING (Style High)
        // スムーズなロールオフ。高いほどプレゼンス（鈴鳴り）を強調。
        let lp_freq = 0.1 + (s_high.powf(0.8) * 0.6);
        let out = x * lp_freq + self.lpf_state[ch] * (1.0 - lp_freq);
        self.lpf_state[ch] = out;

        out
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
        // ODらしい質感を守るため、中ゲイン以上で2倍OS
        let os_factor = if gain > 0.4 { 2 } else { 1 };
        let inv_os = 1.0 / os_factor as f32;

        let mut output_sum = 0.0;
        for i in 0..os_factor {
            let fraction = i as f32 * inv_os;
            let sub = self.os_prev_sub[ch] + (input - self.os_prev_sub[ch]) * fraction;
            output_sum += self.drive_core(sub, gain, s_low, s_mid, s_high, ch);
        }
        self.os_prev_sub[ch] = input;

        let output = output_sum * inv_os;

        // Dry/Wet Mix: Gainが低い時にクリーンの芯を残す（ODの肝）
        let dry_mix = (1.0 - gain).powf(2.5) * 0.5;
        let mixed = output * (1.0 - dry_mix) + input * dry_mix;

        mixed * 0.8
    }
}

impl XrossGainProcessor for XrossDriveSystem {
    fn initialize(
        &mut self,
        layout: &AudioIOLayout,
        _config: &BufferConfig,
        _context: &mut impl InitContext<MetalXross>,
    ) -> bool {
        let num_channels = layout.main_output_channels.map(|n| n.get()).unwrap_or(2) as usize;
        self.low_cut = vec![0.0; num_channels];
        self.mid_shaper = vec![0.0; num_channels];
        self.lpf_state = vec![0.0; num_channels];
        self.prev_input = vec![0.0; num_channels];
        self.os_prev_sub = vec![0.0; num_channels];
        true
    }

    fn process_channel(&mut self, slice: &mut [f32], ch_idx: usize) {
        let g = self.params.general.gain.value();
        let sl = self.params.style.low.value();
        let sm = self.params.style.mid.value();
        let sh = self.params.style.high.value();

        for sample in slice {
            *sample = self.process_sample(*sample, g, sl, sm, sh, ch_idx);
        }
    }
}
