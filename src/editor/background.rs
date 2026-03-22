use nih_plug_vizia::vizia::prelude::*;
use nih_plug_vizia::vizia::vg;

pub struct PcbBackground;

impl PcbBackground {
    pub fn new(cx: &mut Context) -> Handle<'_, Self> {
        Self.build(cx, |_| {})
    }
}

impl View for PcbBackground {
    fn element(&self) -> Option<&'static str> {
        Some("pcb-background")
    }

    fn draw(&self, cx: &mut DrawContext, canvas: &mut Canvas) {
        let bounds = cx.bounds();
        let mut path = vg::Path::new();
        path.rect(bounds.x, bounds.y, bounds.width(), bounds.height());
        canvas.fill_path(&path, &vg::Paint::color(vg::Color::black()));

        // ウィンドウサイズに応じてグリッドの密度を変える
        let area = bounds.width() * bounds.height();
        let grid_size = if area > 600.0 * 400.0 { 20.0 } else { 40.0 };

        let mut paint = vg::Paint::color(vg::Color::hex("111111"));
        paint.set_line_width(1.0);

        // 垂直線
        let mut x = bounds.x;
        while x < bounds.x + bounds.width() {
            let mut p = vg::Path::new();
            p.move_to(x, bounds.y);
            p.line_to(x, bounds.y + bounds.height());
            canvas.stroke_path(&p, &paint);
            x += grid_size;
        }

        // 水平線
        let mut y = bounds.y;
        while y < bounds.y + bounds.height() {
            let mut p = vg::Path::new();
            p.move_to(bounds.x, y);
            p.line_to(bounds.x + bounds.width(), y);
            canvas.stroke_path(&p, &paint);
            y += grid_size;
        }
    }
}
