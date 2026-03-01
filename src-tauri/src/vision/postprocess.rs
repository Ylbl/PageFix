use opencv::{
    core::{self, Mat, Point, Scalar, Size},
    imgproc, photo,
    prelude::*,
};

#[derive(Clone, Copy, Debug)]
struct DenoiseProfile {
    /// NLM filter strength for color image (luminance component).
    color_h: f32,
    /// NLM filter strength for color image (chrominance component).
    color_h_color: f32,
    /// NLM filter strength for grayscale image.
    gray_h: f32,
    template_window: i32,
    search_window: i32,
}

fn denoise_profile(level: &str) -> DenoiseProfile {
    match level.trim().to_ascii_lowercase().as_str() {
        "off" => DenoiseProfile {
            color_h: 0.0,
            color_h_color: 0.0,
            gray_h: 0.0,
            template_window: 7,
            search_window: 21,
        },
        "low" => DenoiseProfile {
            color_h: 3.0,
            color_h_color: 3.0,
            gray_h: 3.0,
            template_window: 7,
            search_window: 15,
        },
        "high" => DenoiseProfile {
            color_h: 8.0,
            color_h_color: 8.0,
            gray_h: 6.0,
            template_window: 7,
            search_window: 21,
        },
        "extreme" => DenoiseProfile {
            color_h: 12.0,
            color_h_color: 12.0,
            gray_h: 10.0,
            template_window: 7,
            search_window: 25,
        },
        // default = medium
        _ => DenoiseProfile {
            color_h: 5.0,
            color_h_color: 5.0,
            gray_h: 4.0,
            template_window: 7,
            search_window: 17,
        },
    }
}

/// Document-scanning post-process pipeline.
///
/// Goal: pure white background, crisp black text, minimal noise.
///
/// Strategy (background-division approach):
///   1. Gentle NLM denoise on color → grayscale
///   2. Estimate page background via large morphological closing
///   3. Divide gray by background → flat-field normalized image (removes shadows/lighting)
///   4. Unsharp-mask to sharpen stroke edges
///   5. Adaptive threshold with small block + low C → preserves thin strokes
///   6. Ensure white-bg / black-text polarity
///   7. Median filter to kill salt-and-pepper noise (edge-preserving, unlike morph-open)
pub(super) fn post_process_sharpen_text_bw(
    warped: &Mat,
    denoise_strength: &str,
) -> Result<Mat, String> {
    let profile = denoise_profile(denoise_strength);

    // --- 1. Gentle color denoise (preserves edges better than aggressive NLM) ---
    let denoised_color = if profile.color_h > 0.0 {
        let mut out = Mat::default();
        photo::fast_nl_means_denoising_colored(
            warped,
            &mut out,
            profile.color_h,
            profile.color_h_color,
            profile.template_window,
            profile.search_window,
        )
        .map_err(|e| format!("彩色降噪失败: {e}"))?;
        out
    } else {
        warped
            .try_clone()
            .map_err(|e| format!("复制图像失败: {e}"))?
    };

    let mut gray = Mat::default();
    imgproc::cvt_color(&denoised_color, &mut gray, imgproc::COLOR_BGR2GRAY, 0)
        .map_err(|e| format!("灰度转换失败: {e}"))?;

    // Optional gray-domain denoise
    let gray = if profile.gray_h > 0.0 {
        let mut out = Mat::default();
        photo::fast_nl_means_denoising(
            &gray,
            &mut out,
            profile.gray_h,
            profile.template_window,
            profile.search_window,
        )
        .map_err(|e| format!("灰度降噪失败: {e}"))?;
        out
    } else {
        gray
    };

    // --- 2. Background estimation via large morphological closing ---
    // The closing kernel must be larger than the thickest text stroke so that
    // text "disappears" and only the page background remains.
    let min_side = gray.cols().min(gray.rows()).max(1);
    let bg_kernel_size = odd_clamp(min_side / 8, 31, 201);
    let bg_kernel = imgproc::get_structuring_element(
        imgproc::MORPH_ELLIPSE,
        Size::new(bg_kernel_size, bg_kernel_size),
        Point::new(-1, -1),
    )
    .map_err(|e| format!("背景核生成失败: {e}"))?;
    let mut background = Mat::default();
    imgproc::morphology_ex(
        &gray,
        &mut background,
        imgproc::MORPH_CLOSE,
        &bg_kernel,
        Point::new(-1, -1),
        1,
        core::BORDER_REPLICATE,
        Scalar::all(0.0),
    )
    .map_err(|e| format!("背景估计失败: {e}"))?;

    // --- 3. Divide: normalized = gray / background * 255 ---
    // This removes uneven lighting and produces a near-white background.
    let mut gray_f = Mat::default();
    gray.convert_to(&mut gray_f, core::CV_32F, 1.0, 0.0)
        .map_err(|e| format!("灰度浮点转换失败: {e}"))?;
    let mut bg_f = Mat::default();
    background
        .convert_to(&mut bg_f, core::CV_32F, 1.0, 0.0)
        .map_err(|e| format!("背景浮点转换失败: {e}"))?;
    // Clamp background to avoid division by zero
    let mut bg_safe = Mat::default();
    imgproc::threshold(&bg_f, &mut bg_safe, 1.0, 0.0, imgproc::THRESH_TOZERO)
        .map_err(|e| format!("背景截断失败: {e}"))?;
    // Where bg_safe == 0, set to 1.0 to avoid div-by-zero
    let mut mask_zero = Mat::default();
    core::compare(&bg_safe, &Scalar::all(0.5), &mut mask_zero, core::CMP_LT)
        .map_err(|e| format!("零值掩码失败: {e}"))?;
    bg_safe
        .set_to(&Scalar::all(1.0), &mask_zero)
        .map_err(|e| format!("零值填充失败: {e}"))?;

    let mut divided = Mat::default();
    core::divide2(&gray_f, &bg_safe, &mut divided, 255.0, -1)
        .map_err(|e| format!("背景除法失败: {e}"))?;
    let mut normalized = Mat::default();
    divided
        .convert_to(&mut normalized, core::CV_8U, 1.0, 0.0)
        .map_err(|e| format!("归一化转换失败: {e}"))?;

    // --- 4. Unsharp mask: sharpen text edges ---
    let mut blurred = Mat::default();
    imgproc::gaussian_blur(
        &normalized,
        &mut blurred,
        Size::new(0, 0),
        2.0,
        2.0,
        core::BORDER_DEFAULT,
    )
    .map_err(|e| format!("锐化模糊失败: {e}"))?;
    let mut sharpened = Mat::default();
    // sharpened = 1.5 * normalized - 0.5 * blurred  (classic unsharp mask)
    core::add_weighted(&normalized, 1.5, &blurred, -0.5, 0.0, &mut sharpened, -1)
        .map_err(|e| format!("锐化失败: {e}"))?;

    // --- 5. Adaptive threshold ---
    // Small block size preserves thin strokes; low C avoids merging nearby glyphs.
    let adapt_block = odd_clamp(min_side / 30, 11, 41);
    let mut binary = Mat::default();
    imgproc::adaptive_threshold(
        &sharpened,
        &mut binary,
        255.0,
        imgproc::ADAPTIVE_THRESH_GAUSSIAN_C,
        imgproc::THRESH_BINARY,
        adapt_block,
        5.0,
    )
    .map_err(|e| format!("自适应阈值失败: {e}"))?;

    // --- 6. Ensure white background / black text ---
    let pixels = i64::from(binary.rows()) * i64::from(binary.cols());
    if pixels > 0 {
        let white =
            core::count_non_zero(&binary).map_err(|e| format!("像素统计失败: {e}"))?;
        // If more than half the pixels are black, the polarity is inverted → flip.
        if i64::from(white) * 2 < pixels {
            let mut inverted = Mat::default();
            core::bitwise_not(&binary, &mut inverted, &core::no_array())
                .map_err(|e| format!("颜色翻转失败: {e}"))?;
            binary = inverted;
        }
    }

    // --- 7. Median filter: remove salt-and-pepper noise without eroding strokes ---
    let mut cleaned = Mat::default();
    imgproc::median_blur(&binary, &mut cleaned, 3)
        .map_err(|e| format!("中值滤波失败: {e}"))?;

    Ok(cleaned)
}

fn odd_clamp(value: i32, min_odd: i32, max_odd: i32) -> i32 {
    let mut v = value.clamp(min_odd, max_odd);
    if v % 2 == 0 {
        if v == max_odd {
            v -= 1;
        } else {
            v += 1;
        }
    }
    v.max(3)
}
