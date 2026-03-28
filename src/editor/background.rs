use nih_plug_egui::egui;
use std::sync::OnceLock;

pub struct PcbBackground;

impl PcbBackground {
    pub fn draw(ui: &mut egui::Ui) {
        let rect = ui.max_rect();
        let painter = ui.painter();

        // 1. テクスチャの取得（初回のみ生成）
        let texture = Self::get_or_init_texture(ui.ctx());

        // 2. "Cover" 計算（アスペクト比を維持して隙間なく埋める）
        let img_size = texture.size_vec2();
        let screen_size = rect.size();

        let scale_x = screen_size.x / img_size.x;
        let scale_y = screen_size.y / img_size.y;
        let scale = scale_x.max(scale_y); // "contain" の場合は min にする

        let draw_size = img_size * scale;
        let draw_rect = egui::Rect::from_center_size(rect.center(), draw_size);

        // 3. 描画
        painter.image(
            texture.id(),
            draw_rect,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            egui::Color32::from_rgba_premultiplied(255, 255, 255, 128),
        );
    }

    /// 画像をバイナリで埋め込み、TextureHandle を返すヘルパー
    fn get_or_init_texture(ctx: &egui::Context) -> &egui::TextureHandle {
        static TEXTURE: OnceLock<egui::TextureHandle> = OnceLock::new();

        TEXTURE.get_or_init(|| {
            // コンパイル時に画像を取り込む
            let image_data = include_bytes!("../../assets/background.png");

            // imageクレートでデコード
            let image = image::load_from_memory(image_data)
                .expect("Failed to load embedded image")
                .to_rgba8();

            let size = [image.width() as usize, image.height() as usize];
            let pixels = image.as_flat_samples();

            let color_image = egui::ColorImage::from_rgba_unmultiplied(size, pixels.as_slice());

            ctx.load_texture("pcb_background", color_image, egui::TextureOptions::LINEAR)
        })
    }
}
