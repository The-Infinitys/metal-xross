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

        // --- 1. インタラクション (変更なし) ---
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
            painter.rect_filled(rect, 2.0, egui::Color32::from_rgb(5, 5, 5));

            // 進捗バー（フィル）
            let fill_rect = if is_vertical {
                let y_pos = rect.bottom() - (visual_val * rect.height());
                egui::Rect::from_min_max(egui::pos2(rect.left(), y_pos), rect.right_bottom())
            } else {
                let x_pos = rect.left() + (visual_val * rect.width());
                egui::Rect::from_min_max(rect.left_top(), egui::pos2(x_pos, rect.bottom()))
            };

            // バーの色を少し明るくして、境界を分かりやすく
            let bar_color = self.color.linear_multiply(0.6);
            painter.rect_filled(fill_rect, 1.0, bar_color);

            // 外枠
            painter.rect_stroke(
                rect,
                2.0,
                egui::Stroke::new(1.0, egui::Color32::from_gray(60)),
                egui::StrokeKind::Inside,
            );

            // テキストの準備
            let text = format!("{}: {}", self.param.name(), self.param);
            let font_id = egui::FontId::proportional(11.0);
            let text_pos = rect.center();

            // --- 視認性向上のためのテキスト描画 ---
            // A. 背景に黒いドロップシャドウを薄く入れる（視認性の確保）
            painter.text(
                text_pos + egui::vec2(1.0, 1.0),
                egui::Align2::CENTER_CENTER,
                &text,
                font_id.clone(),
                egui::Color32::from_black_alpha(200),
            );

            // B. バーの上にある文字を「反転色」にするためのクリッピング描画
            // 1. バーがない部分の文字（グレー）
            painter.with_clip_rect(rect).text(
                text_pos,
                egui::Align2::CENTER_CENTER,
                &text,
                font_id.clone(),
                egui::Color32::from_gray(180),
            );

            // 2. バーと重なっている部分の文字（白または黒）
            // バーの領域だけでクリッピングして、その中だけ明るい色で上書きする
            painter.with_clip_rect(fill_rect).text(
                text_pos,
                egui::Align2::CENTER_CENTER,
                &text,
                font_id,
                egui::Color32::WHITE,
            );

            // つまみ（ハンドル）は最後。細くして文字を邪魔しないように。
            let handle_rect = if is_vertical {
                let y = (rect.bottom() - (visual_val * rect.height()))
                    .clamp(rect.top() + 1.0, rect.bottom() - 1.0);
                egui::Rect::from_center_size(
                    egui::pos2(rect.center().x, y),
                    egui::vec2(rect.width(), 2.0),
                )
            } else {
                let x = (rect.left() + (visual_val * rect.width()))
                    .clamp(rect.left() + 1.0, rect.right() - 1.0);
                egui::Rect::from_center_size(
                    egui::pos2(x, rect.center().y),
                    egui::vec2(2.0, rect.height()),
                )
            };
            painter.rect_filled(handle_rect, 0.0, egui::Color32::WHITE);
        }

        response
    }
}
