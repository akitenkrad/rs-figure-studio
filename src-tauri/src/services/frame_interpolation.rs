use image::RgbaImage;
use crate::error::AppError;

/// フレーム補間方式
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InterpolationMethod {
    /// 前後キーフレームのアルファブレンド
    Crossfade,
    /// 最近傍キーフレームのコピー
    Nearest,
}

impl InterpolationMethod {
    pub fn from_str(s: &str) -> Self {
        match s {
            "nearest" => Self::Nearest,
            _ => Self::Crossfade,
        }
    }
}

/// キーフレーム間を補間してフレーム列を生成
///
/// `keyframes` は `(frame_index, image)` のペア（ソート済み想定）．
/// `total_frames` は出力フレーム数．
/// キーフレーム以外のフレームを指定された方式で補間する．
pub fn interpolate_frames(
    keyframes: &[(u32, RgbaImage)],
    total_frames: u32,
    method: &InterpolationMethod,
) -> Result<Vec<RgbaImage>, AppError> {
    if keyframes.is_empty() {
        return Err(AppError::Validation("キーフレームが空です".into()));
    }

    if total_frames == 0 {
        return Ok(Vec::new());
    }

    let mut result: Vec<Option<RgbaImage>> = (0..total_frames).map(|_| None).collect();

    // Place keyframes
    for (idx, img) in keyframes {
        if (*idx as u32) < total_frames {
            result[*idx as usize] = Some(img.clone());
        }
    }

    // Interpolate missing frames
    for frame_idx in 0..total_frames {
        if result[frame_idx as usize].is_some() {
            continue;
        }

        // Find surrounding keyframes
        let (prev_idx, prev_img) = find_prev_keyframe(keyframes, frame_idx);
        let (next_idx, next_img) = find_next_keyframe(keyframes, frame_idx, total_frames);

        let interpolated = match method {
            InterpolationMethod::Crossfade => {
                if let (Some(prev), Some(next)) = (prev_img, next_img) {
                    let range = next_idx as f32 - prev_idx as f32;
                    let t = if range > 0.0 {
                        (frame_idx as f32 - prev_idx as f32) / range
                    } else {
                        0.0
                    };
                    crossfade_blend(prev, next, t)
                } else if let Some(prev) = prev_img {
                    prev.clone()
                } else if let Some(next) = next_img {
                    next.clone()
                } else {
                    return Err(AppError::Internal("補間に使用できるキーフレームがありません".into()));
                }
            }
            InterpolationMethod::Nearest => {
                // Pick the nearest keyframe
                let nearest = find_nearest_keyframe(keyframes, frame_idx);
                nearest.ok_or_else(|| AppError::Internal("最近傍キーフレームが見つかりません".into()))?.clone()
            }
        };

        result[frame_idx as usize] = Some(interpolated);
    }

    // Unwrap all
    result.into_iter()
        .enumerate()
        .map(|(i, opt)| opt.ok_or_else(|| AppError::Internal(format!("フレーム {} の補間に失敗", i))))
        .collect()
}

fn find_prev_keyframe<'a>(keyframes: &'a [(u32, RgbaImage)], frame_idx: u32) -> (f32, Option<&'a RgbaImage>) {
    let mut best: Option<(u32, &RgbaImage)> = None;
    for (idx, img) in keyframes {
        if *idx <= frame_idx {
            match best {
                Some((best_idx, _)) if *idx > best_idx => best = Some((*idx, img)),
                None => best = Some((*idx, img)),
                _ => {}
            }
        }
    }
    match best {
        Some((idx, img)) => (idx as f32, Some(img)),
        None => (0.0, None),
    }
}

fn find_next_keyframe<'a>(keyframes: &'a [(u32, RgbaImage)], frame_idx: u32, _total: u32) -> (f32, Option<&'a RgbaImage>) {
    let mut best: Option<(u32, &RgbaImage)> = None;
    for (idx, img) in keyframes {
        if *idx > frame_idx {
            match best {
                Some((best_idx, _)) if *idx < best_idx => best = Some((*idx, img)),
                None => best = Some((*idx, img)),
                _ => {}
            }
        }
    }
    match best {
        Some((idx, img)) => (idx as f32, Some(img)),
        None => (frame_idx as f32, None),
    }
}

fn find_nearest_keyframe<'a>(keyframes: &'a [(u32, RgbaImage)], frame_idx: u32) -> Option<&'a RgbaImage> {
    keyframes.iter()
        .min_by_key(|(idx, _)| (*idx as i64 - frame_idx as i64).unsigned_abs())
        .map(|(_, img)| img)
}

/// 2つの RGBA 画像をアルファブレンド（t=0.0 で a, t=1.0 で b）
fn crossfade_blend(a: &RgbaImage, b: &RgbaImage, t: f32) -> RgbaImage {
    let (w, h) = a.dimensions();
    let (bw, bh) = b.dimensions();
    // Use minimum dimensions
    let out_w = w.min(bw);
    let out_h = h.min(bh);

    let mut result = RgbaImage::new(out_w, out_h);
    let t = t.clamp(0.0, 1.0);
    let inv_t = 1.0 - t;

    for y in 0..out_h {
        for x in 0..out_w {
            let pa = a.get_pixel(x, y);
            let pb = b.get_pixel(x, y);
            let r = (pa[0] as f32 * inv_t + pb[0] as f32 * t).round() as u8;
            let g = (pa[1] as f32 * inv_t + pb[1] as f32 * t).round() as u8;
            let b_val = (pa[2] as f32 * inv_t + pb[2] as f32 * t).round() as u8;
            let a_val = (pa[3] as f32 * inv_t + pb[3] as f32 * t).round() as u8;
            result.put_pixel(x, y, image::Rgba([r, g, b_val, a_val]));
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_solid_image(w: u32, h: u32, color: [u8; 4]) -> RgbaImage {
        let mut img = RgbaImage::new(w, h);
        for pixel in img.pixels_mut() {
            *pixel = image::Rgba(color);
        }
        img
    }

    #[test]
    fn test_crossfade_midpoint() {
        let black = make_solid_image(4, 4, [0, 0, 0, 255]);
        let white = make_solid_image(4, 4, [255, 255, 255, 255]);
        let keyframes = vec![(0u32, black), (2u32, white)];
        let result = interpolate_frames(&keyframes, 3, &InterpolationMethod::Crossfade).unwrap();
        assert_eq!(result.len(), 3);
        // Middle frame should be approximately gray (~128)
        let mid = result[1].get_pixel(0, 0);
        assert!((mid[0] as i32 - 128).unsigned_abs() <= 1);
    }

    #[test]
    fn test_nearest_picks_closest() {
        let black = make_solid_image(4, 4, [0, 0, 0, 255]);
        let white = make_solid_image(4, 4, [255, 255, 255, 255]);
        let keyframes = vec![(0u32, black), (4u32, white)];
        let result = interpolate_frames(&keyframes, 5, &InterpolationMethod::Nearest).unwrap();
        assert_eq!(result.len(), 5);
        // Frame 1 should be black (nearest to keyframe 0)
        assert_eq!(result[1].get_pixel(0, 0)[0], 0);
        // Frame 3 should be white (nearest to keyframe 4)
        assert_eq!(result[3].get_pixel(0, 0)[0], 255);
    }

    #[test]
    fn test_all_keyframes_no_interpolation() {
        let r = make_solid_image(2, 2, [255, 0, 0, 255]);
        let g = make_solid_image(2, 2, [0, 255, 0, 255]);
        let b = make_solid_image(2, 2, [0, 0, 255, 255]);
        let keyframes = vec![(0u32, r), (1u32, g), (2u32, b)];
        let result = interpolate_frames(&keyframes, 3, &InterpolationMethod::Crossfade).unwrap();
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].get_pixel(0, 0)[0], 255); // red
        assert_eq!(result[1].get_pixel(0, 0)[1], 255); // green
        assert_eq!(result[2].get_pixel(0, 0)[2], 255); // blue
    }

    #[test]
    fn test_empty_keyframes_error() {
        let result = interpolate_frames(&[], 5, &InterpolationMethod::Crossfade);
        assert!(result.is_err());
    }

    #[test]
    fn test_zero_total_frames() {
        let img = make_solid_image(4, 4, [0, 0, 0, 255]);
        let keyframes = vec![(0u32, img)];
        let result = interpolate_frames(&keyframes, 0, &InterpolationMethod::Crossfade).unwrap();
        assert!(result.is_empty());
    }
}
