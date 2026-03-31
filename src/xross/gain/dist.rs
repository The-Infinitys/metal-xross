use crate::MetalXross;
use crate::params::MetalXrossParams;
use crate::xross::gain::XrossGainProcessor;
use nih_plug::prelude::*;
use std::sync::Arc;

pub struct XrossDistSystem {
    params: Arc<MetalXrossParams>,
    low_cut: Vec<f32>,
    phase_shaper: Vec<f32>,
    stage_states: Vec<[f32; 2]>,
    post_lp: Vec<f32>,
    prev_input: Vec<f32>, // 補間用に追加
}

impl XrossDistSystem {
    pub fn new(params: Arc<MetalXrossParams>) -> Self {
        Self {
            params,
            low_cut: vec![0.0; 2],
            phase_shaper: vec![0.0; 2],
            stage_states: vec![[0.0; 2]; 2],
            post_lp: vec![0.0; 2],
            prev_input: vec![0.0; 2],
        }
    }

    /// 粘りのあるアナログ風サチュレーション
    #[inline(always)]
    fn thick_saturator(&self, x: f32, bias: f32) -> f32 {
        // biasで波形を上下にずらし、偶数次倍音（温かみ）を付与
        let x = x + bias;
        // ソフトな膝（Soft-knee）を持つシグモイド関数的な歪み
        // 1.5 * x / (1.0 + x.abs()) は非常に粘りの強い音になる
        (1.5 * x) / (1.0 + x.abs())
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
        // 1. FAT TIGHTNESS (Pre-HPF)
        // Metalほど削りすぎず、Style Lowで「太いロー」を残せるように
        let hp_freq = 0.02 + (1.0 - s_low).powf(2.0) * 0.15;
        let filtered = input - self.low_cut[ch];
        self.low_cut[ch] = input * hp_freq + self.low_cut[ch] * (1.0 - hp_freq);

        // 2. MIRABASSI PHASE SHAPE (中域の密度)
        // 位相を回して「グワッ」という独特の粘りを作る
        let p_coeff = 0.3 + s_mid * 0.5;
        let phased = p_coeff * filtered + self.phase_shaper[ch];
        self.phase_shaper[ch] = filtered - p_coeff * phased;

        // 3. MULTI-STAGE WARM DRIVE
        // ゲインを大幅強化。指数関数を使い、後半の伸びを確保。
        let drive = (gain * 4.0).exp() * 15.0;
        let mid_push = 1.0 + s_mid * 2.0; // Midでさらに押し出す

        let mut x = phased * drive * mid_push;

        // 第1段階: 非対称サチュレーション (真空管のプリアンプ的)
        x = self.thick_saturator(x, 0.05 + s_mid * 0.1);

        // 第2段階: 強力な圧縮と歪み (パワーアンプ的な粘り)
        // Style Highを「プレゼンス」として使い、歪んだ後の解像度を上げる
        let feedback = 0.3 + (s_high * 0.2);
        x = x * (1.0 - feedback) + self.stage_states[ch][0] * feedback;
        self.stage_states[ch][0] = x;

        x = self.thick_saturator(x * 2.0, -0.03);

        // 4. SOFT POST-FILTER (Style High)
        // 耳に痛い成分だけを落とし、ジューシーな中高域を残す
        let lp_freq = 0.1 + (s_high * 0.6);
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
        let os_factor = if gain > 0.7 {
            4
        } else if gain > 0.3 {
            2
        } else {
            1
        };
        let inv_os = 1.0 / os_factor as f32;

        let mut output_sum = 0.0;
        for i in 0..os_factor {
            // 線形補間で滑らかに
            let fraction = i as f32 * inv_os;
            let sub = self.prev_input[ch] + (input - self.prev_input[ch]) * fraction;
            output_sum += self.drive_core(sub, gain, s_low, s_mid, s_high, ch);
        }
        self.prev_input[ch] = input;

        let output = output_sum * inv_os;

        // 最終メイクアップ: ゲインを上げても極端に音量が変わらないよう補正
        output * (0.5 / (1.0 + gain * 0.8))
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
