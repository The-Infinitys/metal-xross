use nih_plug::prelude::*;

use crate::params::*;
use std::sync::Arc;
mod equalizer;
use equalizer::XrossEqualizer;
mod noise_gate;
use noise_gate::XrossNoiseGate;
mod gain;
use gain::XrossGainSystem;
mod level;
use level::XrossLevelSystem;

pub struct MetalXross {
    params: Arc<MetalXrossParams>,
    equalizer: XrossEqualizer,
    noise_gate: XrossNoiseGate,
    gain: XrossGainSystem,
    level: XrossLevelSystem,
}
impl Default for MetalXross {
    fn default() -> Self {
        Self::new()
    }
}
impl MetalXross {
    pub fn new() -> Self {
        let params = Arc::new(MetalXrossParams::default());
        let equalizer = XrossEqualizer::new(Arc::clone(&params));
        let noise_gate = XrossNoiseGate::new(Arc::clone(&params));
        let gain = XrossGainSystem::new(Arc::clone(&params));
        let level = XrossLevelSystem::new(Arc::clone(&params));
        Self {
            params,
            equalizer,
            noise_gate,
            gain,
            level,
        }
    }
    pub fn params(&self) -> Arc<MetalXrossParams> {
        self.params.clone()
    }
    pub fn process(
        &mut self,
        buffer: &mut Buffer,
        aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        self.level.pre_process(buffer);
        self.noise_gate.process_pre(buffer);
        self.gain.process(buffer, aux, context);
        self.noise_gate.process_post(buffer);
        self.equalizer.process(buffer, aux, context);
        self.level.post_process(buffer);
        ProcessStatus::Normal
    }
    pub fn initialize(
        &mut self,
        audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        context: &mut impl InitContext<Self>,
    ) -> bool {
        let sample_rate = buffer_config.sample_rate;

        // 出力チャンネル数を取得（デフォルトはステレオの2）
        let num_channels = audio_io_layout
            .main_output_channels
            .map(|n| n.get())
            .unwrap_or(2) as usize;

        // 各システムの初期化
        // 1. ノイズゲート
        self.noise_gate.initialize(sample_rate);

        // 2. レベル管理システム（サンプルレートとチャンネル数を渡す）
        self.level.initialize(sample_rate, num_channels);

        // 3. 歪みシステム（内部でチャンネルごとのステートを確保するはず）
        // gain.initialize が bool を返すので、その結果をそのまま利用
        self.gain
            .initialize(audio_io_layout, buffer_config, context)
    }
}
