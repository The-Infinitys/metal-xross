use nih_plug::params::FloatParam;
use nih_plug::prelude::*;
use nih_plug_egui::egui;
use std::f32::consts::PI;

pub struct SingleKnob<'a> {
    param: &'a FloatParam,
    setter: &'a ParamSetter<'a>,
    color: egui::Color32,
}

impl<'a> SingleKnob<'a> {
    pub fn new(param: &'a FloatParam, setter: &'a ParamSetter<'a>, color: egui::Color32) -> Self {
        Self {
            param,
            setter,
            color,
        }
    }
}

impl<'a> egui::Widget for SingleKnob<'a> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let desired_size = egui::vec2(80.0, 100.0);
        let (rect, mut response) = ui.allocate_at_least(
            desired_size,
            egui::Sense::drag().union(egui::Sense::click()),
        );

        // --- 1. 値の管理 (メモリを使用して蓄積) ---
        let id = response.id;
        // ドラッグ中でなければ最新のパラメータ値をメモリに書き込む（同期）
        if !response.dragged() {
            let current_val = self.param.unmodulated_normalized_value();
            ui.memory_mut(|mem| mem.data.insert_temp(id, current_val));
        }

        if response.drag_started() {
            self.setter.begin_set_parameter(self.param);
        }

        if response.dragged() {
            let mut val: f32 = ui.memory(|mem| mem.data.get_temp(id).unwrap_or(0.0));
            // Y軸は上がマイナスなので、マイナスをかけて「上にドラッグ＝プラス」にする
            let delta = -response.drag_delta().y * 0.005;
            val = (val + delta).clamp(0.0, 1.0);

            self.setter.set_parameter_normalized(self.param, val);
            ui.memory_mut(|mem| mem.data.insert_temp(id, val));

            response.mark_changed();
            ui.ctx().request_repaint();
        }

        if response.drag_stopped() {
            self.setter.end_set_parameter(self.param);
        }

        // --- 2. 描画ロジック (上下反転の修正) ---
        if ui.is_rect_visible(rect) {
            let painter = ui.painter();
            let center = rect.center() + egui::vec2(0.0, -10.0);
            let radius = 30.0;

            let val = self.param.unmodulated_normalized_value();

            // 角度の定義:
            // 下を基準にするため、開始を 0.75 * PI (左下)、終了を 2.25 * PI (右下) に設定
            // これで「上が繋がっていて、下が空いている」状態の逆になります。
            let start_angle = PI * 0.75;
            let end_angle = PI * 2.25;
            let current_angle = start_angle + (val * (end_angle - start_angle));

            // 座標変換: x = cos, y = sin (標準的な数学座標系)
            let angle_to_pos = |ang: f32, r: f32| center + egui::vec2(ang.cos(), ang.sin()) * r;

            // 背景の円
            painter.circle_filled(center, radius, egui::Color32::from_rgb(30, 30, 30));

            // 背景の円弧 (空いている部分が下)
            let n_points = 40;
            let bg_points: Vec<egui::Pos2> = (0..=n_points)
                .map(|i| {
                    let a = start_angle + (i as f32 / n_points as f32) * (end_angle - start_angle);
                    angle_to_pos(a, radius + 5.0)
                })
                .collect();
            painter.add(egui::Shape::line(
                bg_points,
                egui::Stroke::new(2.0, egui::Color32::from_gray(60)),
            ));

            // 値の円弧
            if val > 0.0 {
                let val_points: Vec<egui::Pos2> = (0..=n_points)
                    .map(|i| {
                        let a = start_angle
                            + (i as f32 / n_points as f32) * (current_angle - start_angle);
                        angle_to_pos(a, radius + 5.0)
                    })
                    .collect();
                painter.add(egui::Shape::line(
                    val_points,
                    egui::Stroke::new(3.5, self.color),
                ));
            }

            // 指針
            let tip = angle_to_pos(current_angle, radius * 0.9);
            let base = angle_to_pos(current_angle, radius * 0.2);
            painter.line_segment([base, tip], egui::Stroke::new(3.0, self.color));

            // テキスト
            painter.text(
                center + egui::vec2(0.0, radius + 25.0),
                egui::Align2::CENTER_CENTER,
                self.param.to_string(),
                egui::FontId::proportional(14.0),
                egui::Color32::WHITE,
            );
        }

        response
    }
}
