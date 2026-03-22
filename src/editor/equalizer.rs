use nih_plug_vizia::vizia::prelude::*;
use nih_plug_vizia::vizia::vg;

pub struct EqualizerBox;

impl EqualizerBox {
    pub fn new(cx: &mut Context) -> Handle<'_, Self> {
        Self.build(cx, |_| {})
    }
}

impl View for EqualizerBox {
    fn element(&self) -> Option<&'static str> {
        Some("equalizer-box")
    }

    fn draw(&self, cx: &mut DrawContext, canvas: &mut Canvas) {
        let bounds = cx.bounds();
        let is_large = bounds.width() > 500.0;

        let mut paint = vg::Paint::color(vg::Color::hex("080808"));
        let mut path = vg::Path::new();
        path.rect(bounds.x, bounds.y, bounds.width(), bounds.height());
        canvas.fill_path(&path, &paint);

        // 詳細情報の描画（ウィンドウが大きい時のみ）
        if is_large {
            paint.set_color(vg::Color::hex("151515"));
            for i in 1..10 {
                let x = bounds.x + (bounds.width() / 10.0) * i as f32;
                let mut p = vg::Path::new();
                p.move_to(x, bounds.y);
                p.line_to(x, bounds.y + bounds.height());
                canvas.stroke_path(&p, &paint);
            }
        }

        // 共通の枠線
        paint.set_color(vg::Color::hex("333333"));
        canvas.stroke_path(&path, &paint);
    }
}
