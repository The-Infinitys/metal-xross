use nih_plug_egui::egui;

pub struct PcbBackground;

impl PcbBackground {
    pub fn draw(ui: &mut egui::Ui) {
        let rect = ui.max_rect();
        let painter = ui.painter();

        // 黒い背景
        painter.rect_filled(rect, 0.0, egui::Color32::BLACK);

        // ウィンドウサイズに応じてグリッドの密度を変える
        let area = rect.width() * rect.height();
        let grid_size = if area > 600.0 * 400.0 { 20.0 } else { 40.0 };

        let color = egui::Color32::from_rgb(17, 17, 17);
        let stroke = egui::Stroke::new(1.0, color);

        // 垂直線
        let mut x = rect.left();
        while x < rect.right() {
            painter.line_segment(
                [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
                stroke,
            );
            x += grid_size;
        }

        // 水平線
        let mut y = rect.top();
        while y < rect.bottom() {
            painter.line_segment(
                [egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)],
                stroke,
            );
            y += grid_size;
        }
    }
}
