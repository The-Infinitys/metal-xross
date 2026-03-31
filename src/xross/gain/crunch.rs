use crate::MetalXross;
use crate::params::MetalXrossParams;
use crate::xross::gain::XrossGainProcessor;
use nih_plug::prelude::*;
use std::sync::Arc;

pub struct XrossCrunchSystem {
    params: Arc<MetalXrossParams>,
    low_cut_state: Vec<f32>,
    high_shelf_state: Vec<f32>,
    dc_block_state: Vec<[f32; 2]>,
    bias_env: Vec<f32>,
    sag_state: Vec<f32>,
}

impl XrossCrunchSystem {
    pub fn new(params: Arc<MetalXrossParams>) -> Self {
        Self {
            params,
            low_cut_state: vec![0.0; 2],
            high_shelf_state: vec![0.0; 2],
            dc_block_state: vec![[0.0; 2]; 2],
            bias_env: vec![0.0; 2],
            sag_state: vec![1.0; 2],
        }
    }

    /// クランチ特有の「柔らかい」非対称サチュレーション
    #[inline(always)]
    fn warm_tube_stage(&self, x: f32, bias: f32, sag: f32, intensity: f32) -> f32 {
        // intensity=0 の時は完全にリニアな入出力を保証
        if intensity < 0.001 {
            return x;
        }

        let input = (x * sag) + (bias * intensity);

        // プラス側：非常に緩やかなtanh
        // マイナス側：バイアスによってカットオフ気味になる非対称性
        let out = if input > 0.0 {
            input.tanh()
        } else {
            let k = 1.2 + (bias.abs() * 2.0 * intensity);
            (input * k).atan() * (1.0 / k.atan())
        };

        // 原音と歪みをブレンドすることで、gain 0.0 での完全なクリーンを担保
        x * (1.0 - intensity) + out * intensity
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
        // Style Lowが低いほど、中域の「ホコホコ」した温かさを残す
        let hp_freq = 0.01 + (1.0 - s_low).powf(2.0) * 0.12;
        let filtered = input - self.low_cut_state[ch];
        self.low_cut_state[ch] = input * hp_freq + self.low_cut_state[ch] * (1.0 - hp_freq);

        // 2. DYNAMIC SAG & BIAS (呼吸するアンプ)
        let abs_in = filtered.abs();

        // Sag: 弾いた瞬間に「クッ」と沈み込むコンプレッション
        let target_sag = 1.0 - (abs_in * 0.3 * gain);
        self.sag_state[ch] += (target_sag - self.sag_state[ch]) * 0.05;

        // Bias: グリッド・バイアス変動による偶数次倍音の付加
        let target_bias = -(abs_in * 0.2 * gain) + (s_mid * 0.05);
        self.bias_env[ch] += (target_bias - self.bias_env[ch]) * 0.01;

        // 3. DRIVE STAGE
        // gain=0.0 では drive=1.0、intensity=0.0 になるよう設計
        let drive = 1.0 + (gain * 12.0);
        let intensity = gain.min(1.0); // 歪みの深さ自体を制御

        // 多段処理だが、intensity によって gain=0 時のバイパスを実現
        let stage1 = self.warm_tube_stage(
            filtered * drive,
            self.bias_env[ch],
            self.sag_state[ch],
            intensity,
        );
        let stage2 = self.warm_tube_stage(stage1 * 1.2, self.bias_env[ch] * 0.5, 1.0, intensity);

        // 4. POST-FILTER (Style High)
        // Style Highが高いほど、高域の「鈴鳴り（Glassy High）」を開放
        let lp_freq = 0.1 + (s_high.powf(1.5) * 0.7);
        let out = stage2 * lp_freq + self.high_shelf_state[ch] * (1.0 - lp_freq);
        self.high_shelf_state[ch] = out;

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
        // Gain 0.0 の時は全ての処理をバイパスして完全にクリーンにする
        if gain < 0.0001 {
            return input;
        }

        // クランチは繊細な質感が命。常に2倍オーバーサンプリング
        let mut output = 0.0;
        for _ in 0..2 {
            output += self.drive_core(input, gain, s_low, s_mid, s_high, ch);
        }
        output *= 0.5;

        // DC Block
        let final_out = output - self.dc_block_state[ch][0] + (0.995 * self.dc_block_state[ch][1]);
        self.dc_block_state[ch][0] = output;
        self.dc_block_state[ch][1] = final_out;

        // gainに応じてわずかに音量を補正
        final_out * (1.0 - gain * 0.15)
    }
}

impl XrossGainProcessor for XrossCrunchSystem {
    fn initialize(
        &mut self,
        layout: &AudioIOLayout,
        _config: &BufferConfig,
        _context: &mut impl InitContext<MetalXross>,
    ) -> bool {
        let num_channels = layout.main_output_channels.map(|n| n.get()).unwrap_or(2) as usize;
        self.low_cut_state = vec![0.0; num_channels];
        self.high_shelf_state = vec![0.0; num_channels];
        self.dc_block_state = vec![[0.0; 2]; num_channels];
        self.bias_env = vec![0.0; num_channels];
        self.sag_state = vec![1.0; num_channels];
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
