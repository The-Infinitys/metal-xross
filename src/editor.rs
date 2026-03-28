use nih_plug::params::FloatParam;
use nih_plug_egui::{
    create_egui_editor,
    egui::{self, Color32},
};
use std::sync::Arc;

use crate::params::MetalXrossParams;

pub mod background;
pub mod equalizer;
pub mod knob;

use background::PcbBackground;
use equalizer::EqualizerBox;
use knob::SingleKnob;
use knob::StackedKnob;
/// HSLからColor32を生成するヘルパー (虹色用)
/// h, s, l は 0.0..=1.0
fn from_hsl(h: f32, s: f32, l: f32) -> Color32 {
    let rgb = egui::epaint::Hsva::new(h, s, l, 1.0).to_rgba_unmultiplied();
    let r = (rgb[0] * 255.0) as u8;
    let g = (rgb[1] * 255.0) as u8;
    let b = (rgb[2] * 255.0) as u8;
    Color32::from_rgb(r, g, b)
}

/// パラメータの表示設定を管理する構造体
struct KnobConfig<'a> {
    label: &'static str,
    param: KnobParam<'a>,
}
enum KnobParam<'a> {
    Stacked(&'a FloatParam, &'a FloatParam),
    Single(&'a FloatParam),
}

pub fn create(params: Arc<MetalXrossParams>) -> Option<Box<dyn nih_plug::prelude::Editor>> {
    create_egui_editor(
        params.editor_state.clone(),
        (),
        |_cx, _state| {},
        move |egui_ctx, setter, _state| {
            egui::CentralPanel::default()
                .frame(egui::Frame::new().fill(Color32::BLACK))
                .show(egui_ctx, |ui| {
                    // 背景の描画
                    PcbBackground::draw(ui);

                    // --- パラメータリストの定義 ---
                    // ここに並べたい順番で登録するだけでOKです
                    let knob_configs = [
                        KnobConfig {
                            label: "INPUT",
                            param: KnobParam::Stacked(
                                &params.general.input.gain,
                                &params.general.input.limit,
                            ),
                        },
                        KnobConfig {
                            label: "GAIN",
                            param: KnobParam::Single(&params.general.gain),
                        },
                        KnobConfig {
                            label: "STYLE",
                            param: KnobParam::Single(&params.style.kind),
                        },
                        KnobConfig {
                            label: "LOW",
                            param: KnobParam::Single(&params.style.low),
                        },
                        KnobConfig {
                            label: "MID",
                            param: KnobParam::Single(&params.style.mid),
                        },
                        KnobConfig {
                            label: "HIGH",
                            param: KnobParam::Single(&params.style.high),
                        },
                        KnobConfig {
                            label: "OUTPUT",
                            param: KnobParam::Stacked(
                                &params.general.output.gain,
                                &params.general.output.limit,
                            ),
                        },
                    ];

                    ui.vertical_centered(|ui| {
                        ui.add_space(20.0);

                        // ノブの並びを動的に生成
                        ui.horizontal(|ui| {
                            let total_knobs = knob_configs.len();
                            let spacing = 30.0;
                            ui.spacing_mut().item_spacing.x = spacing;

                            // 中央寄せを計算するために全体の幅を確保
                            ui.columns(total_knobs, |columns| {
                                for (i, config) in knob_configs.iter().enumerate() {
                                    columns[i].vertical_centered(|ui| {
                                        // 虹色の計算 (色相 H を 0.0 ~ 0.8 くらいで回すと綺麗です)
                                        let hue = i as f32 / total_knobs as f32;
                                        let color = from_hsl(hue, 0.8, 0.6);

                                        ui.label(
                                            egui::RichText::new(config.label)
                                                .size(13.0)
                                                .strong()
                                                .color(Color32::WHITE),
                                        );

                                        ui.add_space(4.0);
                                        match config.param {
                                            KnobParam::Single(param) => {
                                                ui.add(SingleKnob::new(param, setter, color));
                                            }
                                            KnobParam::Stacked(param1, param2) => {
                                                ui.add(StackedKnob::new(
                                                    param1, param2, setter, color, color,
                                                ));
                                            }
                                        }
                                    });
                                }
                            });
                        });

                        ui.add_space(30.0);
                        EqualizerBox::draw(ui, &params, setter);
                    });
                });
        },
    )
}
