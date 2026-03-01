use opencv::{
    core::{self, Mat, Point, Scalar, Size},
    imgproc, photo,
    prelude::*,
};

#[derive(Clone, Copy, Debug)]
struct DenoiseProfile {
    color_h_primary: f32,
    color_h_color_primary: f32,
    color_secondary: Option<(f32, f32)>,
    gray_h: f32,
    template_window: i32,
    search_window: i32,
}

fn denoise_profile(level: &str) -> DenoiseProfile {
    match level.trim().to_ascii_lowercase().as_str() {
        "off" => DenoiseProfile {
            color_h_primary: 0.0,
            color_h_color_primary: 0.0,
            color_secondary: None,
            gray_h: 0.0,
            template_window: 7,
            search_window: 21,
        },
        "low" => DenoiseProfile {
            color_h_primary: 6.0,
            color_h_color_primary: 6.0,
            color_secondary: None,
            gray_h: 4.0,
            template_window: 7,
            search_window: 15,
        },
        "high" => DenoiseProfile {
            color_h_primary: 14.0,
            color_h_color_primary: 14.0,
            color_secondary: Some((9.0, 9.0)),
            gray_h: 10.0,
            template_window: 7,
            search_window: 27,
        },
        "extreme" => DenoiseProfile {
            color_h_primary: 20.0,
            color_h_color_primary: 20.0,
            color_secondary: Some((14.0, 14.0)),
            gray_h: 14.0,
            template_window: 7,
            search_window: 31,
        },
        _ => DenoiseProfile {
            color_h_primary: 10.0,
            color_h_color_primary: 10.0,
            color_secondary: None,
            gray_h: 7.0,
            template_window: 7,
            search_window: 21,
        },
    }
}

pub(super) fn post_process_sharpen_text_bw(warped: &Mat, denoise_strength: &str) -> Result<Mat, String> {
    let profile = denoise_profile(denoise_strength);
    let color_ready = if profile.color_h_primary > 0.0 {
        let mut color_primary = Mat::default();
        photo::fast_nl_means_denoising_colored(
            warped,
            &mut color_primary,
            profile.color_h_primary,
            profile.color_h_color_primary,
            profile.template_window,
            profile.search_window,
        )
        .map_err(|e| format!("后处理彩色 NLM 降噪失败: {e}"))?;
        if let Some((h, h_color)) = profile.color_secondary {
            let mut color_second = Mat::default();
            photo::fast_nl_means_denoising_colored(
                &color_primary,
                &mut color_second,
                h,
                h_color,
                profile.template_window,
                profile.search_window,
            )
            .map_err(|e| format!("后处理二次彩色 NLM 降噪失败: {e}"))?;
            color_second
        } else {
            color_primary
        }
    } else {
        warped
            .try_clone()
            .map_err(|e| format!("后处理复制图像失败: {e}"))?
    };
    let mut gray = Mat::default();
    imgproc::cvt_color(&color_ready, &mut gray, imgproc::COLOR_BGR2GRAY, 0)
        .map_err(|e| format!("后处理灰度转换失败: {e}"))?;
    let gray_ready = if profile.gray_h > 0.0 {
        let mut gray_denoised = Mat::default();
        photo::fast_nl_means_denoising(
            &gray,
            &mut gray_denoised,
            profile.gray_h,
            profile.template_window,
            profile.search_window,
        )
        .map_err(|e| format!("后处理灰度 NLM 降噪失败: {e}"))?;
        gray_denoised
    } else {
        gray
    };
    let min_side = gray_ready.cols().min(gray_ready.rows()).max(1);
    let mut clahe = imgproc::create_clahe(3.0, Size::new(8, 8))
        .map_err(|e| format!("后处理 CLAHE 创建失败: {e}"))?;
    let mut contrast = Mat::default();
    clahe
        .apply(&gray_ready, &mut contrast)
        .map_err(|e| format!("后处理 CLAHE 失败: {e}"))?;
    let mut blur = Mat::default();
    imgproc::gaussian_blur(
        &contrast,
        &mut blur,
        Size::new(0, 0),
        0.9,
        0.9,
        core::BORDER_DEFAULT,
    )
    .map_err(|e| format!("后处理平滑失败: {e}"))?;
    let mut sharpened = Mat::default();
    core::add_weighted(&contrast, 1.7, &blur, -0.7, 0.0, &mut sharpened, -1)
        .map_err(|e| format!("后处理锐化失败: {e}"))?;
    let adapt_block = odd_clamp(min_side / 18, 21, 57);
    let mut adaptive = Mat::default();
    imgproc::adaptive_threshold(
        &sharpened,
        &mut adaptive,
        255.0,
        imgproc::ADAPTIVE_THRESH_GAUSSIAN_C,
        imgproc::THRESH_BINARY,
        adapt_block,
        9.0,
    )
    .map_err(|e| format!("后处理自适应阈值失败: {e}"))?;
    let mut otsu = Mat::default();
    imgproc::threshold(
        &sharpened,
        &mut otsu,
        0.0,
        255.0,
        imgproc::THRESH_BINARY | imgproc::THRESH_OTSU,
    )
    .map_err(|e| format!("后处理 OTSU 阈值失败: {e}"))?;
    let adaptive_score = binary_text_score(&adaptive)?;
    let otsu_score = binary_text_score(&otsu)?;
    let mut binary = if adaptive_score >= otsu_score {
        adaptive
    } else {
        otsu
    };
    let pixels = i64::from(binary.rows()) * i64::from(binary.cols());
    if pixels > 0 {
        let white = core::count_non_zero(&binary).map_err(|e| format!("后处理统计失败: {e}"))?;
        if i64::from(white) * 2 < pixels {
            let mut inverted = Mat::default();
            core::bitwise_not(&binary, &mut inverted, &core::no_array())
                .map_err(|e| format!("后处理颜色翻转失败: {e}"))?;
            binary = inverted;
        }
    }
    let kernel =
        imgproc::get_structuring_element(imgproc::MORPH_RECT, Size::new(2, 2), Point::new(-1, -1))
            .map_err(|e| format!("后处理核生成失败: {e}"))?;
    let mut cleaned = Mat::default();
    imgproc::morphology_ex(
        &binary,
        &mut cleaned,
        imgproc::MORPH_OPEN,
        &kernel,
        Point::new(-1, -1),
        1,
        core::BORDER_DEFAULT,
        Scalar::all(0.0),
    )
    .map_err(|e| format!("后处理降噪失败: {e}"))?;
    Ok(cleaned)
}

fn binary_text_score(binary: &Mat) -> Result<f64, String> {
    let pixels = i64::from(binary.rows()) * i64::from(binary.cols());
    if pixels <= 0 {
        return Ok(0.0);
    }
    let white = core::count_non_zero(binary).map_err(|e| format!("后处理统计失败: {e}"))?;
    let black_ratio = (pixels - i64::from(white)) as f64 / pixels as f64;
    let target = 0.14_f64;
    let score = 1.0 - ((black_ratio - target).abs() / target.max(0.01));
    Ok(score.clamp(0.0, 1.0))
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
