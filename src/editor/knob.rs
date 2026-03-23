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
        let (rect, mut response) = ui.allocate_at_least(desired_size, egui::Sense::drag());

        let id = response.id;

        // --- 1. 値の同期ロジック ---

        // ドラッグ中かどうかで「表示に使う値」を切り替える
        let visual_val = if response.dragged() {
            // ドラッグ中は、メモリに保存されている「ユーザーが動かした最新の値」を使う
            let mut val: f32 = ui
                .memory(|mem| mem.data.get_temp(id))
                .unwrap_or_else(|| self.param.unmodulated_normalized_value());

            let delta = -response.drag_delta().y * 0.005;
            if delta != 0.0 {
                val = (val + delta).clamp(0.0, 1.0);

                // ホストへ通知（ドラッグ中は set_parameter_normalized のみ！）
                self.setter.set_parameter_normalized(self.param, val);

                // 次のフレームのためにメモリに保存
                ui.memory_mut(|mem| mem.data.insert_temp(id, val));
            }
            val
        } else {
            // ドラッグしていない時は、DAW側の値をそのまま使う
            let val = self.param.unmodulated_normalized_value();
            // メモリも同期しておく
            ui.memory_mut(|mem| mem.data.insert_temp(id, val));
            val
        };

        // A. ドラッグ開始の瞬間だけ begin を呼ぶ
        if response.drag_started() {
            self.setter.begin_set_parameter(self.param);
        }

        // B. ドラッグ終了の瞬間だけ end を呼ぶ
        if response.drag_stopped() {
            self.setter.end_set_parameter(self.param);
        }

        // --- 2. 描画ロジック ---
        if ui.is_rect_visible(rect) {
            let painter = ui.painter();
            let center = rect.center() + egui::vec2(0.0, -10.0);
            let radius = 30.0;

            let start_angle = PI * 0.75;
            let end_angle = PI * 2.25;
            // 決定した visual_val を使って角度を計算
            let current_angle = start_angle + (visual_val * (end_angle - start_angle));

            let angle_to_pos = |ang: f32, r: f32| center + egui::vec2(ang.cos(), ang.sin()) * r;

            // 背景描画
            painter.circle_filled(center, radius, egui::Color32::from_rgb(15, 15, 15));
            painter.circle_stroke(
                center,
                radius,
                egui::Stroke::new(1.0, egui::Color32::from_gray(60)),
            );

            // インジケータ
            let n_points = 40;
            if visual_val > 0.0 {
                let current_n = (n_points as f32 * visual_val).ceil() as usize;
                let val_points: Vec<egui::Pos2> = (0..=current_n)
                    .map(|i| {
                        let a =
                            start_angle + (i as f32 / n_points as f32) * (end_angle - start_angle);
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
            painter.line_segment([base, tip], egui::Stroke::new(2.5, self.color));

            // パラメータ名の表示
            painter.text(
                center + egui::vec2(0.0, radius + 25.0),
                egui::Align2::CENTER_CENTER,
                self.param.to_string(),
                egui::FontId::proportional(12.0),
                egui::Color32::WHITE,
            );
        }

        if response.dragged() {
            ui.ctx().request_repaint();
        }
        println!(
            "{}: {}",
            self.param.name(),
            self.param.unmodulated_normalized_value()
        );
        response
    }
}
