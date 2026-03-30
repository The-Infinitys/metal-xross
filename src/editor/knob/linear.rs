use nih_plug::params::FloatParam;
use nih_plug::prelude::*;
use nih_plug_egui::egui;

pub struct LinearSlider<'a> {
    param: &'a FloatParam,
    setter: &'a ParamSetter<'a>,
    color: egui::Color32,
}

impl<'a> LinearSlider<'a> {
    pub fn new(param: &'a FloatParam, setter: &'a ParamSetter<'a>, color: egui::Color32) -> Self {
        Self {
            param,
            setter,
            color,
        }
    }
}

impl<'a> egui::Widget for LinearSlider<'a> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let desired_size = egui::vec2(120.0, 24.0);
        let (rect, response) = ui.allocate_at_least(desired_size, egui::Sense::click_and_drag());

        let is_vertical = rect.height() > rect.width();

        // --- 1. インタラクション ---
        if response.drag_started() {
            self.setter.begin_set_parameter(self.param);
        }

        if response.dragged() {
            let val = self.param.unmodulated_normalized_value();
            let delta = if is_vertical {
                -response.drag_delta().y / rect.height()
            } else {
                response.drag_delta().x / rect.width()
            };

            if delta != 0.0 {
                let new_val = (val + delta).clamp(0.0, 1.0);
                self.setter.set_parameter_normalized(self.param, new_val);
            }
        }

        if response.drag_stopped() {
            self.setter.end_set_parameter(self.param);
        }

        if response.double_clicked() {
            self.setter.begin_set_parameter(self.param);
            self.setter
                .set_parameter_normalized(self.param, self.param.default_normalized_value());
            self.setter.end_set_parameter(self.param);
        }

        // --- 2. 描画ロジック ---
        if ui.is_rect_visible(rect) {
            let painter = ui.painter();
            let visual_val = self.param.unmodulated_normalized_value();

            // 背景（溝）
            painter.rect_filled(rect, 2.0, egui::Color32::from_rgb(10, 10, 10));

            // 引数エラーの修正: 第4引数に StrokeKind::Inside を追加
            painter.rect_stroke(
                rect,
                2.0,
                egui::Stroke::new(1.0, egui::Color32::from_gray(50)),
                egui::StrokeKind::Inside,
            );

            // フィル（進捗バー）
            let fill_rect = if is_vertical {
                let y_pos = rect.bottom() - (visual_val * rect.height());
                egui::Rect::from_min_max(egui::pos2(rect.left(), y_pos), rect.right_bottom())
            } else {
                let x_pos = rect.left() + (visual_val * rect.width());
                egui::Rect::from_min_max(rect.left_top(), egui::pos2(x_pos, rect.bottom()))
            };
            painter.rect_filled(fill_rect.shrink(1.0), 1.0, self.color.linear_multiply(0.4));

            // ハンドル（つまみ）
            let (handle_rect, line_start, line_end) = if is_vertical {
                let y = (rect.bottom() - (visual_val * rect.height()))
                    .clamp(rect.top() + 2.0, rect.bottom() - 2.0);
                let r = egui::Rect::from_center_size(
                    egui::pos2(rect.center().x, y),
                    egui::vec2(rect.width(), 4.0),
                );
                (
                    r,
                    egui::pos2(r.left(), r.center().y),
                    egui::pos2(r.right(), r.center().y),
                )
            } else {
                let x = (rect.left() + (visual_val * rect.width()))
                    .clamp(rect.left() + 2.0, rect.right() - 2.0);
                let r = egui::Rect::from_center_size(
                    egui::pos2(x, rect.center().y),
                    egui::vec2(4.0, rect.height()),
                );
                (
                    r,
                    egui::pos2(r.center().x, r.top()),
                    egui::pos2(r.center().x, r.bottom()),
                )
            };

            painter.rect_filled(handle_rect, 1.0, egui::Color32::WHITE);
            painter.line_segment(
                [line_start, line_end],
                egui::Stroke::new(1.0, egui::Color32::BLACK),
            );

            // テキスト
            let text = format!("{}: {}", self.param.name(), self.param);
            painter.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                text,
                egui::FontId::proportional(11.0),
                egui::Color32::WHITE,
            );
        }

        response
    }
}
