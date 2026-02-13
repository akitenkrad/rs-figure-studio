use anyhow::Result;
use image::GenericImageView;
use ndarray::Array4;
use ort::session::builder::GraphOptimizationLevel;
use ort::session::Session;
use ort::value::Tensor;
use std::path::Path;

const INPUT_SIZE: u32 = 320;
const INPUT_SIZE_USIZE: usize = INPUT_SIZE as usize;
const MEAN: [f32; 3] = [0.485, 0.456, 0.406];
const STD: [f32; 3] = [0.229, 0.224, 0.225];

/// ONNX Runtime を使用した背景除去サービス
///
/// U2-Net モデルで Salient Object Detection を行い，
/// 前景/背景を分離してアルファチャンネル付き PNG を出力する．
pub struct OnnxService {
    session: Session,
}

// OnnxService は Session を保持するだけなので Send + Sync を手動実装
// ort::Session は内部的にスレッドセーフな C++ ランタイムを使用
unsafe impl Send for OnnxService {}
unsafe impl Sync for OnnxService {}

impl OnnxService {
    /// モデルファイルから ONNX セッションを作成
    pub fn new(model_path: &Path) -> Result<Self> {
        let session = Session::builder()?
            .with_optimization_level(GraphOptimizationLevel::Level3)?
            .with_intra_threads(4)?
            .commit_from_file(model_path)?;

        log::info!("ONNX session created: {:?}", model_path);
        log::info!(
            "Input names: {:?}",
            session
                .inputs()
                .iter()
                .map(|i| i.name())
                .collect::<Vec<_>>()
        );
        log::info!(
            "Output names: {:?}",
            session
                .outputs()
                .iter()
                .map(|o| o.name())
                .collect::<Vec<_>>()
        );

        Ok(Self { session })
    }

    /// 単一画像の背景除去
    pub fn remove_background(&mut self, input_path: &str, output_path: &str) -> Result<()> {
        self.remove_background_with_threshold(input_path, output_path, 0.5)
    }

    /// 閾値指定での背景除去
    pub fn remove_background_with_threshold(
        &mut self,
        input_path: &str,
        output_path: &str,
        threshold: f32,
    ) -> Result<()> {
        // 1. 画像読み込み
        let original = image::open(input_path)?;

        // 2. 前処理: ndarray -> ort Tensor
        let input_array = preprocess(&original);
        let input_tensor = Tensor::from_array(input_array)?;

        // 3. ONNX 推論
        let outputs = self.session.run(ort::inputs![input_tensor])?;

        // 4. 出力テンソル取得（最初の出力を使用）
        let (shape, data) = outputs[0].try_extract_tensor::<f32>()?;

        // shape は [1, 1, 320, 320] を期待
        let expected_elements = 1 * 1 * INPUT_SIZE_USIZE * INPUT_SIZE_USIZE;
        if data.len() < expected_elements {
            anyhow::bail!(
                "Unexpected output tensor size: {} (expected at least {}), shape: {:?}",
                data.len(),
                expected_elements,
                shape
            );
        }

        // 5. 後処理
        let result = postprocess(&original, data, threshold);

        // 6. 出力ディレクトリ作成 & 保存
        if let Some(parent) = Path::new(output_path).parent() {
            std::fs::create_dir_all(parent)?;
        }
        result.save(output_path)?;

        log::info!("Background removed: {} -> {}", input_path, output_path);
        Ok(())
    }

    /// バッチ背景除去（逐次処理 + プログレスコールバック）
    pub fn remove_background_batch(
        &mut self,
        inputs: &[(String, String)],
        threshold: f32,
        on_progress: impl Fn(usize, usize),
    ) -> Result<Vec<String>> {
        let total = inputs.len();
        let mut results = Vec::with_capacity(total);

        for (i, (input_path, output_path)) in inputs.iter().enumerate() {
            self.remove_background_with_threshold(input_path, output_path, threshold)?;
            results.push(output_path.clone());
            on_progress(i + 1, total);
        }

        Ok(results)
    }
}

/// 画像を前処理して ONNX 入力テンソルに変換
///
/// 処理手順:
/// 1. 320x320 にリサイズ（Bilinear フィルタ）
/// 2. RGB に変換
/// 3. [0, 255] -> [0.0, 1.0] に正規化
/// 4. ImageNet 統計量でチャンネル正規化
/// 5. NCHW 形式 [1, 3, 320, 320] のテンソルに変換
fn preprocess(img: &image::DynamicImage) -> Array4<f32> {
    let resized = img.resize_exact(
        INPUT_SIZE,
        INPUT_SIZE,
        image::imageops::FilterType::Triangle, // Bilinear
    );

    let rgb = resized.to_rgb8();

    let mut input = Array4::<f32>::zeros((1, 3, INPUT_SIZE_USIZE, INPUT_SIZE_USIZE));

    for y in 0..INPUT_SIZE_USIZE {
        for x in 0..INPUT_SIZE_USIZE {
            let pixel = rgb.get_pixel(x as u32, y as u32);
            for c in 0..3 {
                let normalized = (pixel[c] as f32 / 255.0 - MEAN[c]) / STD[c];
                input[[0, c, y, x]] = normalized;
            }
        }
    }

    input
}

/// ONNX 出力マスクを後処理し，背景除去済み画像を生成
///
/// 処理手順:
/// 1. マスクデータ（フラット配列）を [320, 320] のグレースケール画像に変換
/// 2. 元画像サイズにリサイズ（Bilinear フィルタ）
/// 3. 閾値処理: 閾値未満は完全透明，閾値以上はマスク値をアルファとして適用
/// 4. RGBA PNG として返却
fn postprocess(
    original: &image::DynamicImage,
    mask_data: &[f32],
    threshold: f32,
) -> image::RgbaImage {
    let (orig_w, orig_h) = original.dimensions();

    // マスクを [320, 320] のグレースケール画像に変換
    // mask_data は [1, 1, 320, 320] のフラット配列
    let mut mask_img = image::GrayImage::new(INPUT_SIZE, INPUT_SIZE);
    for y in 0..INPUT_SIZE_USIZE {
        for x in 0..INPUT_SIZE_USIZE {
            let idx = y * INPUT_SIZE_USIZE + x;
            let value = mask_data[idx].clamp(0.0, 1.0);
            mask_img.put_pixel(x as u32, y as u32, image::Luma([(value * 255.0) as u8]));
        }
    }

    // 元画像サイズにリサイズ
    let resized_mask = image::imageops::resize(
        &mask_img,
        orig_w,
        orig_h,
        image::imageops::FilterType::Triangle,
    );

    // 元画像を RGBA に変換
    let mut rgba = original.to_rgba8();

    // マスクをアルファチャンネルとして適用
    for y in 0..orig_h {
        for x in 0..orig_w {
            let mask_value = resized_mask.get_pixel(x, y)[0] as f32 / 255.0;
            let pixel = rgba.get_pixel_mut(x, y);

            if mask_value < threshold {
                // 背景: 完全透明
                pixel[3] = 0;
            } else {
                // 前景: マスク値に応じたアルファ（ソフトエッジ）
                pixel[3] = (mask_value * 255.0) as u8;
            }
        }
    }

    rgba
}
