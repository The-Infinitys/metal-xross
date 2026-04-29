use egui::{pos2, Color32, Context, Rect, TextureHandle, TextureOptions, Ui};
use std::sync::OnceLock;

pub struct PcbBackground;

impl PcbBackground {
    /// 背景を描画します。中央揃えの "Cover"（充填）表示を行います。
    pub fn draw(ui: &mut Ui) {
        // 現在のパネルやウィンドウの有効領域全体を取得
        let rect = ui.max_rect();
        let painter = ui.painter();

        // 1. テクスチャの取得（未生成なら生成）
        let texture = Self::get_or_init_texture(ui.ctx());

        // 2. 表示領域の計算 (Aspect Fill / Cover)
        let img_size = texture.size_vec2();
        let screen_size = rect.size();

        // アスペクト比を維持しながら、画面を隙間なく埋めるスケールを計算
        let scale = (screen_size.x / img_size.x).max(screen_size.y / img_size.y);
        let draw_size = img_size * scale;

        // 画面中央に配置（はみ出した分はクリップされます）
        let draw_rect = Rect::from_center_size(rect.center(), draw_size);

        // 3. 描画
        // メタル系プラグインの重厚感を出すため、少し暗めのグレーを乗算(Multiply)気味に適用
        // op = 255 で原画そのまま、値を下げると暗くなります
        let op = 160;
        let tint = Color32::from_gray(op);

        painter.image(
            texture.id(),
            draw_rect,
            Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), // UV座標全体
            tint,
        );
    }

    /// 画像をロードし、テクスチャをキャッシュして返します。
    fn get_or_init_texture(ctx: &Context) -> &TextureHandle {
        static TEXTURE: OnceLock<TextureHandle> = OnceLock::new();

        TEXTURE.get_or_init(|| {
            // コンパイル時にアセットをバイナリとして埋め込み
            let image_data = include_bytes!("../../assets/background.png");

            // image クレートを使用してデコード
            let image = image::load_from_memory(image_data)
                .expect("Failed to load embedded background.png. Check assets folder.")
                .to_rgba8();

            let size = [image.width() as usize, image.height() as usize];
            let pixels = image.as_flat_samples();

            let color_image = egui::ColorImage::from_rgba_unmultiplied(size, pixels.as_slice());

            // wgpuバックエンドで滑らかに表示されるよう LINEAR フィルタを適用
            ctx.load_texture("pcb_background", color_image, TextureOptions::LINEAR)
        })
    }
}
