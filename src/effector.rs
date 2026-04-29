use std::sync::Arc;
use truce::prelude::*;

use crate::params::MetalXrossParams;
mod equalizer;
mod gain;
mod level;
mod noise_gate;

use equalizer::XrossEqualizer;
use gain::XrossGainSystem;
use level::XrossLevelSystem;
use noise_gate::XrossNoiseGate;

pub struct MetalXross {
    params: Arc<MetalXrossParams>,
    equalizer: XrossEqualizer,
    noise_gate: XrossNoiseGate,
    gain: XrossGainSystem,
    level: XrossLevelSystem,
}

impl MetalXross {
    pub fn new(params: Arc<MetalXrossParams>) -> Self {
        Self {
            params: Arc::clone(&params),
            equalizer: XrossEqualizer::new(Arc::clone(&params)),
            noise_gate: XrossNoiseGate::new(Arc::clone(&params)),
            gain: XrossGainSystem::new(Arc::clone(&params)),
            level: XrossLevelSystem::new(Arc::clone(&params)),
        }
    }

    /// truce の初期化メソッド。サンプルレート変更時などに呼ばれる
    pub fn reset(&mut self, sr: f64, _bs: usize) {
        let sample_rate = sr as f32;
        let num_channels = 2; // truceのデフォルト。動的に変える場合はbufferから取得

        // 各サブシステムの初期化
        self.params.set_sample_rate(sr);
        self.params.snap_smoothers();

        self.noise_gate.initialize(sample_rate, num_channels);
        self.level.initialize(sample_rate, num_channels);
        self.equalizer.initialize(sample_rate, num_channels);
        // gain.rs の initialize が引数を要求する場合に合わせる
        self.gain.initialize(sample_rate, num_channels);
    }

    pub fn process(
        &mut self,
        buffer: &mut AudioBuffer,
        _events: &EventList,
        _context: &mut ProcessContext,
    ) -> ProcessStatus {
        // 1. Pre-Gain Level & Gate (入力処理)
        self.level.pre_process_buffer(buffer);
        self.noise_gate.process_pre_buffer(buffer);

        // 2. Main Gain System (歪み核心部)
        // GainSystem内部で各チャンネルの process_channel を呼ぶ
        self.gain.process_buffer(buffer);

        // 3. Post-Gain Gate & EQ & Level (仕上げ)
        self.noise_gate.process_post_buffer(buffer);
        self.equalizer.process_buffer(buffer);
        self.level.post_process_buffer(buffer);

        ProcessStatus::Normal
    }

    pub fn params(&self) -> Arc<MetalXrossParams> {
        self.params.clone()
    }

    pub fn ui(&self) -> Box<dyn Editor> {
        crate::editor::create(self.params())
    }
}
