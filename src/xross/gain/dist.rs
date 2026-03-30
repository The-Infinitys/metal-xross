use crate::MetalXross;
use crate::params::MetalXrossParams;
use crate::xross::gain::XrossGainProcessor;
use nih_plug::prelude::*;
use std::sync::Arc;

pub struct XrossDistSystem {
    params: Arc<MetalXrossParams>,
    // 状態保持用
    low_cut: Vec<f32>,
    phase_shaper: Vec<f32>, // Mirabassi的位相操作用
    stage_states: Vec<[f32; 2]>,
    post_lp: Vec<f32>,
}

impl XrossDistSystem {
    pub fn new(params: Arc<MetalXrossParams>) -> Self {
        Self {
            params,
            low_cut: vec![0.0; 2],
            phase_shaper: vec![0.0; 2],
            stage_states: vec![[0.0; 2]; 2],
            post_lp: vec![0.0; 2],
        }
    }

    /// ディストーションらしい、少し丸みのあるクリッピング
    #[inline(always)]
    fn soft_dist_clip(&self, x: f32, skew: f32) -> f32 {
        let x = x + skew; // わずかな非対称性
        let abs_x = x.abs();
        if abs_x < 0.25 {
            x
        } else {
            // 指数関数的な飽和 (よりアナログに近いカーブ)
            let sign = x.signum();
            sign * (0.25 + 0.7 * (1.0 - (-(abs_x - 0.25) * 2.0).exp()))
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
        // 1. TIGHTNESS (Pre-HPF)
        // s_lowが高いほどローを残すが、歪みすぎを防ぐため 120Hz付近は常に軽くカット
        let hp_freq = 0.04 + (1.0 - s_low).powf(1.5) * 0.25;
        let filtered = input - self.low_cut[ch];
        self.low_cut[ch] = input * hp_freq + self.low_cut[ch] * (1.0 - hp_freq);

        // 2. MIRABASSI PHASE SHAPE (中域の粘り)
        // Style Midに連動して、400Hz-1kHz付近の位相を回す
        let p_coeff = 0.2 + s_mid * 0.4;
        let phased = p_coeff * filtered + self.phase_shaper[ch];
        self.phase_shaper[ch] = filtered - p_coeff * phased;

        // 3. MULTI-STAGE DRIVE
        // ゲイン設定: 0.0でも真空管をドライブしたような質感が出るように。
        let drive = 5.0 + gain * 45.0;
        let mid_impact = 0.8 + s_mid * 1.2;

        let mut x = phased * drive * mid_impact;

        // 第1段階: 緩やかな歪み
        x = self.soft_dist_clip(x, 0.02);
        x = x * 0.7 + self.stage_states[ch][0] * 0.3;
        self.stage_states[ch][0] = x;

        // 第2段階: 追い打ちの歪み（ここで音の壁を作る）
        x = self.soft_dist_clip(x * 1.5, -0.01);
        x = x * 0.8 + self.stage_states[ch][1] * 0.2;
        self.stage_states[ch][1] = x;

        // 4. POST-FILTER (Style High)
        // s_highが高いほど、高域の「ジリジリ」を「ザラッ」とした質感に変える
        let lp_freq = 0.08 + (s_high.powf(1.3) * 0.5);
        let out = x * lp_freq + self.post_lp[ch] * (1.0 - lp_freq);
        self.post_lp[ch] = out;

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
        // ゲインに応じてOS倍率を自動変更
        let os_factor = if gain > 0.8 {
            4
        } else if gain > 0.4 {
            2
        } else {
            1
        };

        let mut output = 0.0;
        if os_factor == 1 {
            output = self.drive_core(input, gain, s_low, s_mid, s_high, ch);
        } else {
            for _ in 0..os_factor {
                output += self.drive_core(input, gain, s_low, s_mid, s_high, ch);
            }
            output /= os_factor as f32;
        }

        // 最終メイクアップ: 歪ませても音量が一定になるように
        output * (0.6 / (1.0 + gain * 0.5))
    }
}

impl XrossGainProcessor for XrossDistSystem {
    fn initialize(
        &mut self,
        layout: &AudioIOLayout,
        _config: &BufferConfig,
        _context: &mut impl InitContext<MetalXross>,
    ) -> bool {
        let num_channels = layout.main_output_channels.map(|n| n.get()).unwrap_or(2) as usize;
        self.low_cut = vec![0.0; num_channels];
        self.phase_shaper = vec![0.0; num_channels];
        self.stage_states = vec![[0.0; 2]; num_channels];
        self.post_lp = vec![0.0; num_channels];
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
