use crate::MetalXross;
use crate::params::MetalXrossParams;
use crate::xross::gain::XrossGainProcessor;
use nih_plug::prelude::*;
use std::sync::Arc;

pub struct XrossDriveSystem {
    params: Arc<MetalXrossParams>,
    low_cut: Vec<f32>,
    mid_shaper: Vec<f32>, // Mirabassi的位相操作（ドライブ用）
    lpf_state: Vec<f32>,
    prev_input: Vec<f32>, // 微分成分用（エッジの強調）
}

impl XrossDriveSystem {
    pub fn new(params: Arc<MetalXrossParams>) -> Self {
        Self {
            params,
            low_cut: vec![0.0; 2],
            mid_shaper: vec![0.0; 2],
            lpf_state: vec![0.0; 2],
            prev_input: vec![0.0; 2],
        }
    }

    /// 真空管の特性に近いソフトサチュレーション
    /// x=0付近はリニアで、1.0に近づくにつれて滑らかに圧縮する
    #[inline(always)]
    fn tube_saturate(&self, x: f32) -> f32 {
        let abs_x = x.abs();
        if abs_x < 0.4 {
            x
        } else {
            // atanやtanhよりも「肩」が柔らかいシグモイド曲線
            let sign = x.signum();
            sign * (0.4 + 0.55 * ((abs_x - 0.4) / 0.55).tanh())
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
        // 1. INPUT SHAPING (Transient Edge)
        // ピッキングの瞬間だけわずかにゲインを上げる（微分成分のブレンド）
        let diff = input - self.prev_input[ch];
        self.prev_input[ch] = input;
        let edged_input = input + diff * 0.2 * (1.0 - gain * 0.5);

        // 2. TIGHTNESS (Style Low)
        // ODは中域が主役。s_lowが低いほど720Hz付近を強調するバンドパス的な挙動に
        let hp_freq = 0.03 + (1.0 - s_low).powf(1.2) * 0.2;
        let filtered = edged_input - self.low_cut[ch];
        self.low_cut[ch] = edged_input * hp_freq + self.low_cut[ch] * (1.0 - hp_freq);

        // 3. PHASE SHIFT (Style Mid: Presence)
        // 歪む前に中域の位相を回し、和音を弾いた時の「分離感」を作る
        let p_coeff = 0.1 + s_mid * 0.4;
        let phased = p_coeff * filtered + self.mid_shaper[ch];
        self.mid_shaper[ch] = filtered - p_coeff * phased;

        // 4. MULTI-STAGE SOFT DRIVE
        // Gain 0.0 = クリーンブースター / Gain 1.0 = 激しいオーバードライブ
        let drive_amount = 1.0 + (gain * 35.0);
        let mut x = phased * drive_amount;

        // Stage 1: 非対称性を少し加え、偶数次倍音（温かみ）を出す
        x = self.tube_saturate(x + 0.03);

        // Stage 2: さらに深く突っ込む（s_midで2段目の歪み量を調整）
        let s2_gain = 1.2 + s_mid * 0.8;
        x = self.tube_saturate(x * s2_gain);

        // 5. TONE CONTROL (Style High)
        // ODらしいクリーミーな高域。s_highが高いと高域の「鈴鳴り」を追加
        let lp_freq = 0.12 + s_high * 0.5;
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
        // ODはエイリアシングが目立ちにくいため、高ゲイン時のみ2倍OS
        let os_factor = if gain > 0.6 { 2 } else { 1 };

        let mut output = 0.0;
        for _ in 0..os_factor {
            output += self.drive_core(input, gain, s_low, s_mid, s_high, ch);
        }
        output /= os_factor as f32;

        // Dry/Wet Mix: Gainが低いときは元のクリーンを混ぜて「芯」を残す
        let dry_mix = (1.0 - gain).powf(2.0) * 0.4;
        let mixed = output * (1.0 - dry_mix) + input * dry_mix;

        // 出力メイクアップ
        mixed * 0.75
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
