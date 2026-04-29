use truce::{params::FloatParam, Params};

#[derive(Params)]
pub struct MetalXrossParams {
    // --- Noise Gate ---
    #[param(
        name = "Gate Threshold",
        range = "linear(-70.0, -10.0)",
        default = -45.0,
        unit = " dB",
        smooth = "exp(50)"
    )]
    pub gate_threshold: FloatParam,

    #[param(
        name = "Gate Tolerance",
        range = "linear(0.0, 1.0)",
        default = 0.5,
        smooth = "exp(50)"
    )]
    pub gate_tolerance: FloatParam,

    #[param(
            name = "Gate Release",
            // 一旦 linear で修正。1.0ms から 500.0ms の範囲
            range = "linear(1.0, 500.0)",
            default = 100.0,
            unit = " ms",
            smooth = "exp(50)"
        )]
    pub gate_release: FloatParam,
    // --- General / Gain ---
    #[param(
        name = "Input Gain",
        range = "linear(-60.0, 12.0)",
        default = 0.0,
        unit = " dB",
        smooth = "exp(50)"
    )]
    pub input_gain: FloatParam,

    #[param(
        name = "Input Limit",
        range = "linear(-60.0, 12.0)",
        default = 0.0,
        unit = " dB",
        smooth = "exp(50)"
    )]
    pub input_limit: FloatParam,

    #[param(
        name = "Drive Gain",
        range = "linear(0.0, 1.0)",
        default = 0.5,
        smooth = "exp(50)"
    )]
    pub gain: FloatParam,

    #[param(
        name = "Output Gain",
        range = "linear(-60.0, 0.0)",
        default = -3.0,
        unit = " dB",
        smooth = "exp(50)"
    )]
    pub output_gain: FloatParam,

    #[param(
        name = "Output Limit",
        range = "linear(-60.0, 0.0)",
        default = -3.0,
        unit = " dB",
        smooth = "exp(50)"
    )]
    pub output_limit: FloatParam,

    // --- Style ---
    #[param(
        name = "Style",
        range = "linear(0, 3)",
        default = 3.0,
        // 種類を選択するものなのでスムージングは不要な場合が多い
        smooth = "none"
    )]
    pub style_kind: FloatParam,

    #[param(
        name = "Style Low",
        range = "linear(0.0, 1.0)",
        default = 0.5,
        smooth = "exp(50)"
    )]
    pub style_low: FloatParam,

    #[param(
        name = "Style Mid",
        range = "linear(0.0, 1.0)",
        default = 0.5,
        smooth = "exp(50)"
    )]
    pub style_mid: FloatParam,

    #[param(
        name = "Style High",
        range = "linear(0.0, 1.0)",
        default = 0.5,
        smooth = "exp(50)"
    )]
    pub style_high: FloatParam,

    // --- Equalizer ---
    // Low Band
    #[param(
        name = "EQ Low Freq",
        // 周波数は対数(skewed)が必須。低域にノブの余裕を持たせる
        range = "skewed(20.0, 2000.0, 0.5)",
        default = 100.0,
        unit = " Hz",
        smooth = "exp(50)"
    )]
    pub eq_lo_freq: FloatParam,
    #[param(
        name = "EQ Low Q",
        range = "skewed(0.1, 10.0, 0.5)",
        default = 0.707,
        smooth = "exp(50)"
    )]
    pub eq_lo_q: FloatParam,
    #[param(
        name = "EQ Low Gain",
        range = "linear(-20.0, 20.0)",
        default = 0.0,
        unit = " dB",
        smooth = "exp(50)"
    )]
    pub eq_lo_gain: FloatParam,

    // Mid Band
    #[param(
        name = "EQ Mid Freq",
        range = "skewed(200.0, 8000.0, 0.5)",
        default = 1000.0,
        unit = " Hz",
        smooth = "exp(50)"
    )]
    pub eq_mi_freq: FloatParam,
    #[param(
        name = "EQ Mid Q",
        range = "skewed(0.1, 10.0, 0.5)",
        default = 0.707,
        smooth = "exp(50)"
    )]
    pub eq_mi_q: FloatParam,
    #[param(
        name = "EQ Mid Gain",
        range = "linear(-20.0, 20.0)",
        default = 0.0,
        unit = " dB",
        smooth = "exp(50)"
    )]
    pub eq_mi_gain: FloatParam,

    // High Band
    #[param(
        name = "EQ High Freq",
        range = "skewed(1000.0, 20000.0, 0.5)",
        default = 4000.0,
        unit = " Hz",
        smooth = "exp(50)"
    )]
    pub eq_hi_freq: FloatParam,
    #[param(
        name = "EQ High Q",
        range = "skewed(0.1, 10.0, 0.5)",
        default = 0.707,
        smooth = "exp(50)"
    )]
    pub eq_hi_q: FloatParam,
    #[param(
        name = "EQ High Gain",
        range = "linear(-20.0, 20.0)",
        default = 0.0,
        unit = " dB",
        smooth = "exp(50)"
    )]
    pub eq_hi_gain: FloatParam,
}
