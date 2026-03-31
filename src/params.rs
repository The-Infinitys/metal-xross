use nih_plug::prelude::*;
use nih_plug_egui::EguiState;
use std::sync::Arc;

#[derive(Params)]
pub struct MetalXrossParams {
    /// エディタの状態
    #[persist = "editor_state"]
    pub editor_state: Arc<EguiState>,

    #[nested(group = "NoiseGate", id_prefix = "noise_gate_")]
    pub noise_gate: NoiseGateParams,
    #[nested(group = "General")]
    pub general: GeneralParams,
    #[nested(group = "Style Settings", id_prefix = "style_")]
    pub style: StyleParams,

    // --- 下段に並べるEQ ---
    #[nested(group = "Equalizer", id_prefix = "eq_")]
    pub eq: EqualizerParams,
}
#[derive(Params)]
pub struct NoiseGateParams {
    /// ゲートが開く音量の閾値 (dB)
    /// -60dB (ガバガバ) 〜 -10dB (パツパツ)
    #[id = "thr"]
    pub threshold: FloatParam,

    /// スペクトル解析の感度 (Tolerance)
    /// 0.0: 倍音を最大限保護 (自然) / 1.0: ノイズっぽければ即座に高域カット (超タイト)
    #[id = "tol"]
    pub tolerance: FloatParam,

    /// ゲートが閉じる速度 (Release)
    /// 1ms (Djentな刻み) 〜 500ms (自然な余韻)
    #[id = "rel"]
    pub release: FloatParam,
}

impl Default for NoiseGateParams {
    fn default() -> Self {
        let noise_gate_string_func = |v: f32| format!("{:.1}", v);
        Self {
            threshold: FloatParam::new(
                "Threshold",
                -45.0,
                FloatRange::Linear {
                    min: -70.0,
                    max: -10.0,
                },
            )
            .with_value_to_string(Arc::new(noise_gate_string_func))
            .with_unit(" dB"),

            tolerance: FloatParam::new("Tolerance", 0.5, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_value_to_string(Arc::new(noise_gate_string_func)),
            release: FloatParam::new(
                "Release",
                100.0,
                FloatRange::Skewed {
                    min: 1.0,
                    max: 500.0,
                    factor: FloatRange::skew_factor(-2.0), // 速い方に解像度を寄せる
                },
            )
            .with_value_to_string(Arc::new(noise_gate_string_func))
            .with_unit(" ms"),
        }
    }
}
#[derive(Params)]
pub struct GeneralParams {
    #[nested(group = "Input", id_prefix = "input_")]
    pub input: LevelParams,
    #[id = "gain"]
    pub gain: FloatParam,
    #[nested(group = "Output", id_prefix = "output_")]
    pub output: LevelParams,
}
impl Default for GeneralParams {
    fn default() -> Self {
        let gain_string_func = |v: f32| format!("{:.2}", v);
        Self {
            input: LevelParams::new(1.0, 1.0, 2.0),
            gain: FloatParam::new("Gain", 0.5, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_value_to_string(Arc::new(gain_string_func)),
            output: LevelParams::new(1.0, 0.5, 1.0),
        }
    }
}
#[derive(Params)]
pub struct LevelParams {
    #[id = "limit"]
    pub limit: FloatParam,
    #[id = "gain"]
    pub gain: FloatParam,
}
impl LevelParams {
    pub fn new(default_gain: f32, default_limit: f32, max_limit: f32) -> Self {
        let db_string_func = Arc::new(|v: f32| {
            if v <= 0.00001 {
                "-inf".to_string()
            } else {
                format!("{:.1}", 20.0 * v.log10())
            }
        });

        // --- Gainの範囲設定 ---
        // 0.0倍(-inf)から4.0倍(+12dB)まで。中心を1.0(0dB)に据える
        let gain_range = FloatRange::SymmetricalSkewed {
            min: 0.0,
            max: 4.0,
            factor: FloatRange::skew_factor(2.0),
            center: 1.0,
        };

        // --- Limitの範囲設定 ---
        // 要件: default_limit がノブの真ん中（center）に来るように設定
        // メタル用途なら min は 0.01 (-40dB) 程度、max は max_limit (例: 1.0)
        let limit_range = FloatRange::SymmetricalSkewed {
            min: 0.001, // 完全な0だとlogが壊れるため、-60dB程度を最小値に
            max: max_limit,
            center: default_limit,
            factor: FloatRange::skew_factor((default_limit.ln() / 0.5_f32.ln()).abs().max(0.1)),
        };

        Self {
            limit: FloatParam::new("Limit", default_limit, limit_range)
                .with_value_to_string(db_string_func.clone()),
            gain: FloatParam::new("Gain", default_gain, gain_range)
                .with_value_to_string(db_string_func),
        }
    }
}
#[derive(Params)]
pub struct StyleParams {
    #[id = "kind"]
    pub kind: FloatParam,
    #[id = "pre_low"]
    pub low: FloatParam,
    #[id = "pre_mid"]
    pub mid: FloatParam,
    #[id = "pre_high"]
    pub high: FloatParam,
}

impl Default for StyleParams {
    fn default() -> Self {
        let style_string_func = |v: f32| format!("{:.2}", v);

        Self {
            kind: FloatParam::new("Style", 3.0, FloatRange::Linear { min: 0.0, max: 3.0 })
                .with_value_to_string(Arc::new(|v| {
                    if v < 0.5 {
                        "Crunch"
                    } else if v < 1.5 {
                        "Drive"
                    } else if v < 2.5 {
                        "Distortion"
                    } else {
                        "Metal"
                    }
                    .to_string()
                })),
            low: FloatParam::new("Style Low", 0.5, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_value_to_string(Arc::new(style_string_func)),
            mid: FloatParam::new("Style Mid", 0.5, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_value_to_string(Arc::new(style_string_func)),
            high: FloatParam::new("Style High", 0.5, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_value_to_string(Arc::new(style_string_func)),
        }
    }
}

impl Default for MetalXrossParams {
    fn default() -> Self {
        Self {
            editor_state: EguiState::from_size(800, 500),
            noise_gate: NoiseGateParams::default(),
            general: GeneralParams::default(),
            style: StyleParams::default(),
            eq: EqualizerParams::default(),
        }
    }
}

// --- EQ関連の構造体はそのまま利用可能 ---

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
            low: PeqBandParams::new("Low", 100.0, 20.0, 20000.0),
            mid: PeqBandParams::new("Mid", 1000.0, 20.0, 20000.0),
            high: PeqBandParams::new("High", 4000.0, 20.0, 8000.0),
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
    pub fn new(name: &str, default_freq: f32, min_freq: f32, max_freq: f32) -> Self {
        Self {
            freq: FloatParam::new(
                format!("{} Freq", name),
                default_freq,
                FloatRange::SymmetricalSkewed {
                    min: min_freq,
                    max: max_freq,
                    factor: FloatRange::skew_factor(-2.0),
                    center: default_freq,
                },
            )
            .with_unit(" Hz")
            .with_value_to_string(Arc::new(|v| format!("{:.1}", v))),

            q: FloatParam::new(
                format!("{} Q", name),
                0.707,
                FloatRange::Skewed {
                    min: 0.1,
                    max: 10.0,
                    factor: FloatRange::skew_factor(-1.0),
                },
            ),

            gain: FloatParam::new(
                format!("{} Gain", name),
                0.0,
                FloatRange::SymmetricalSkewed {
                    min: -20.0,
                    max: 20.0,
                    factor: 1.0,
                    center: 0.0,
                },
            )
            .with_unit(" dB")
            .with_value_to_string(Arc::new(|v| format!("{v:+.1}"))),
        }
    }
}
