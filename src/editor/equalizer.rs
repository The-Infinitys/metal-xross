use nih_plug_egui::egui;

pub struct EqualizerBox;

impl EqualizerBox {
    pub fn draw(ui: &mut egui::Ui) {
        let (rect, _response) = ui.allocate_at_least(
            egui::vec2(ui.available_width(), 150.0),
            egui::Sense::hover(),
        );

        let painter = ui.painter();
        let is_large = rect.width() > 500.0;

        // 背景
        painter.rect_filled(rect, 0.0, egui::Color32::from_rgb(8, 8, 8));

        // 詳細情報の描画（ウィンドウが大きい時のみ）
        if is_large {
            let detail_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(21, 21, 21));
            for i in 1..10 {
                let x = rect.left() + (rect.width() / 10.0) * i as f32;
                painter.line_segment(
                    [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
                    detail_stroke,
                );
            }
        }

        // 共通の枠線
        painter.rect(
            rect,
            0.0,
            egui::Color32::TRANSPARENT,
            egui::Stroke::new(1.0, egui::Color32::from_rgb(51, 51, 51)),
            egui::StrokeKind::Middle,
        );
    }
}
