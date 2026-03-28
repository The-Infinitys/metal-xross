use crate::MetalXross;
use crate::params::MetalXrossParams;
use crate::xross::gain::XrossGainProcessor;
use nih_plug::prelude::*;
use std::sync::Arc;

pub struct XrossDriveSystem {
    params: Arc<MetalXrossParams>,
    low_cut: Vec<f32>,
    mid_focus: Vec<f32>,
    lpf_state: Vec<f32>,
}

impl XrossDriveSystem {
    pub fn new(params: Arc<MetalXrossParams>) -> Self {
        Self {
            params,
            low_cut: vec![0.0; 2],
            mid_focus: vec![0.0; 2],
            lpf_state: vec![0.0; 2],
        }
    }

    /// オーバードライブの心臓部：粘りを生む非線形関数
    /// 多項式ソフトクリップに少しの「肩」を持たせ、サステインを稼ぐ
    fn drive_clip(&self, x: f32) -> f32 {
        let abs_x = x.abs();
        if abs_x < 0.5 {
            x // リニア領域
        } else if abs_x < 1.0 {
            // 0.5~1.0の間で滑らかに飽和（粘り）
            x.signum() * (abs_x - 0.5 * (abs_x - 0.5).powi(2) / 0.5)
        } else {
            // 限界値付近でのコンプレッション感
            x.signum() * 0.875
        }
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
        // --- 1. MID-PUSH & LOW-CUT (Driveの骨格) ---
        // Style Lowが高いほど低域を残し、低いほどタイトに（中域にフォーカス）
        let hp_freq = 0.05 + (1.0 - s_low).powf(1.5) * 0.25;
        let pre_filtered = input - self.low_cut[ch];
        self.low_cut[ch] = input * hp_freq + self.low_cut[ch] * (1.0 - hp_freq);

        // 中域の「粘り」を出すためのバンドパス的なブースト
        let mid_boost = (s_mid + 0.5) * 1.5;
        let focused = pre_filtered * mid_boost;

        // --- 2. DYNAMIC GAIN (サステインの演出) ---
        // Gain 0でも1.2倍程度の内部ゲインを持たせ、サチュレーションの入り口を作る
        // Gainを上げると30倍近くまで跳ね上がり、ドライブ感が増す
        let internal_drive = 1.2 + gain * 25.0;
        let mut x = focused * internal_drive;

        // --- 3. SOFT CLIPPING STAGES ---
        // 2段階でクリップさせることで、より複雑な倍音とサステインを生む
        x = self.drive_clip(x);
        x = self.drive_clip(x * 1.5); // 2段目の突っ込み

        // --- 4. TONE & PRESENCE (Style High) ---
        // 高域の抜けを調整。Style Highが高いほどエッジが立ち、低いとクリーミーに
        let lp_freq = 0.15 + s_high * 0.6;
        let smoothed = x * lp_freq + self.lpf_state[ch] * (1.0 - lp_freq);
        self.lpf_state[ch] = smoothed;

        // --- 5. MAKEUP & OUTPUT ---
        // 歪みによる音量の増加を抑えつつ、中域の密度を維持
        let makeup = 0.7 / (1.0 + gain * 1.2);

        // 元の入力成分をわずかにブレンド（Gain 0での芯を残すため）
        let dry_blend = 0.3 * (1.0 - gain).max(0.0);

        (smoothed * (1.0 - dry_blend) + input * dry_blend) * makeup
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
        self.mid_focus = vec![0.0; num_channels];
        self.lpf_state = vec![0.0; num_channels];
        true
    }

    fn process_channel(&mut self, slice: &mut [f32], ch_idx: usize) {
        let gain = self.params.general.gain.value();
        let s_low = self.params.style.low.value();
        let s_mid = self.params.style.mid.value();
        let s_high = self.params.style.high.value();

        for sample in slice {
            *sample = self.process_sample(*sample, gain, s_low, s_mid, s_high, ch_idx);
        }
    }
}
