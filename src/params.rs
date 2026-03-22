use nih_plug_vizia::ViziaState;
use std::sync::Arc;

use nih_plug::{
    params::{FloatParam, Params},
    prelude::FloatRange,
};
#[derive(Params)]
pub struct MetalXrossParams {
    /// The editor state.
    #[persist = "editor_state"]
    pub editor_state: Arc<ViziaState>,

    /// The gain of the effect.
    #[id = "mx_gain"]
    pub gain: FloatParam,
    /// The volume of the effect.
    #[id = "mx_lvl"]
    pub level: FloatParam,
    /// The style of the effect.
    #[id = "mx_styl"]
    pub style: FloatParam,
    /// The tightness of the effect.
    #[id = "mx_tght"]
    pub tight: FloatParam,
    /// The brightness of the effect.
    #[id = "mx_brgh"]
    pub bright: FloatParam,

    #[nested(group = "Equalizer", id_prefix = "eq_")]
    pub eq: EqualizerParams,
}

impl Default for MetalXrossParams {
    fn default() -> Self {
        // 1. Distortion Style (0.0: Crunch ~ 3.0: Metal)
        let style = FloatParam::new(
            "Style",
            3.0, // デフォルトはフルメタル
            FloatRange::Linear { min: 0.0, max: 3.0 },
        )
        .with_value_to_string(Arc::new(|v| {
            if v < 0.5 {
                "Crunch".to_string()
            } else if v < 1.5 {
                "Drive".to_string()
            } else if v < 2.5 {
                "Distortion".to_string()
            } else {
                "Metal".to_string()
            }
        }));

        // 2. Tight (ローカット。上げるほど低域がスッキリして刻みがシャープになる)
        let tight = FloatParam::new(
            "Tight",
            0.5, // ほどよくタイト
            FloatRange::Linear { min: 0.0, max: 1.0 },
        );

        // 3. Bright (高域の抜け。上げるほどジャリッとする)
        let bright = FloatParam::new("Bright", 0.5, FloatRange::Linear { min: 0.0, max: 1.0 });

        let gain = FloatParam::new(
            "Gain",
            0.0,
            FloatRange::Skewed {
                min: 0.0,
                max: 1.0,
                factor: FloatRange::skew_factor(-0.2),
            },
        );
        let level = FloatParam::new("Level", 0.0, FloatRange::Linear { min: 0.0, max: 2.0 });
        Self {
            editor_state: ViziaState::new(|| (800, 500)),
            style,
            tight,
            bright,
            gain,
            level,
            eq: EqualizerParams::default(),
        }
    }
}
#[derive(Params)]
pub struct PeqBandParams {
    #[id = "freq"]
    pub freq: FloatParam,
    #[id = "q"]
    pub q: FloatParam,
    #[id = "gain"]
    pub gain: FloatParam,
}

impl PeqBandParams {
    /// 役割（Low, Midなど）に合わせてデフォルト値を変えて生成できるようにする
    pub fn new(name: &str, default_freq: f32, min_freq: f32, max_freq: f32) -> Self {
        Self {
            freq: FloatParam::new(
                format!("{} Freq", name), // 名前を自動生成
                default_freq,
                FloatRange::SymmetricalSkewed {
                    min: min_freq,
                    max: max_freq,
                    factor: FloatRange::skew_factor(-2.0),
                    center: default_freq,
                },
            )
            .with_unit(" Hz")
            // 小数点以下を整理して表示をスッキリさせる
            .with_value_to_string(Arc::new(|v| format!("{:.1}", v))),

            q: FloatParam::new(
                format!("{} Q", name),
                0.707,
                // Q値は対数的に変化させたほうが「広がる・狭まる」感覚に一致します
                FloatRange::Skewed {
                    min: 0.1,
                    max: 10.0,
                    factor: FloatRange::skew_factor(-1.0),
                },
            ),

            gain: FloatParam::new(
                format!("{} Gain", name),
                0.0,
                // センター（0dB）を真ん中に固定し、±の操作感を均等にする
                FloatRange::SymmetricalSkewed {
                    min: -20.0,
                    max: 20.0,
                    factor: 1.0,
                    center: 0.0,
                },
            )
            .with_unit(" dB")
            .with_value_to_string(Arc::new(|v| format!("{v:+.1}"))), // +記号を表示
        }
    }
}
#[derive(Params)]
pub struct EqualizerParams {
    #[nested(group = "Low Band", id_prefix = "lo_")]
    pub low: PeqBandParams,

    #[nested(group = "Mid Band", id_prefix = "mi_")]
    pub mid: PeqBandParams,

    #[nested(group = "High Band", id_prefix = "hi_")]
    pub high: PeqBandParams,
}

impl Default for EqualizerParams {
    fn default() -> Self {
        Self {
            low: PeqBandParams::new("Low", 100.0, 20.0, 200.0),
            mid: PeqBandParams::new("Mid", 1000.0, 20.0, 2000.0),
            high: PeqBandParams::new("High", 4000.0, 2000.0, 8000.0),
        }
    }
}
