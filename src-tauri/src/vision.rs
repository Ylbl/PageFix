#[cfg(target_os = "linux")]
use base64::{engine::general_purpose::STANDARD, Engine as _};
#[cfg(target_os = "linux")]
use opencv::{
    core::{self, Mat, Point, Point2f, Scalar, Size, Vector},
    imgcodecs, imgproc, photo,
    prelude::*,
};

#[cfg(target_os = "linux")]
use crate::PolygonPoint;

// OpenCV-based detection + perspective correction pipeline used by Rust commands.
#[cfg(target_os = "linux")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct PointI {
    x: i32,
    y: i32,
}

#[cfg(target_os = "linux")]
pub(crate) fn looks_like_jpeg(bytes: &[u8]) -> bool {
    bytes.len() >= 4
        && bytes[0] == 0xFF
        && bytes[1] == 0xD8
        && bytes[bytes.len() - 2] == 0xFF
        && bytes[bytes.len() - 1] == 0xD9
}

#[cfg(target_os = "linux")]
pub(crate) fn detect_polygon_on_jpeg_linux(jpeg_bytes: &[u8]) -> Option<Vec<PolygonPoint>> {
    let input = Vector::<u8>::from_slice(jpeg_bytes);
    let color_full = imgcodecs::imdecode(&input, imgcodecs::IMREAD_COLOR).ok()?;
    if color_full.empty() {
        return None;
    }

    let full_width = color_full.cols();
    let full_height = color_full.rows();
    if full_width < 12 || full_height < 12 {
        return None;
    }

    let mut color = Mat::default();
    let max_dim = 960_i32;
    let max_side = full_width.max(full_height);

    if max_side > max_dim {
        let ratio = f64::from(max_dim) / f64::from(max_side);
        let scaled_width = (f64::from(full_width) * ratio).round() as i32;
        let scaled_height = (f64::from(full_height) * ratio).round() as i32;

        imgproc::resize(
            &color_full,
            &mut color,
            Size::new(scaled_width.max(1), scaled_height.max(1)),
            0.0,
            0.0,
            imgproc::INTER_AREA,
        )
        .ok()?;
    } else {
        color = color_full;
    }

    let quad = detect_document_quad_opencv(&color)?;

    let width_f = color.cols() as f32;
    let height_f = color.rows() as f32;
    if width_f <= 1.0 || height_f <= 1.0 {
        return None;
    }

    let mut points = Vec::with_capacity(4);
    for p in quad {
        points.push(PolygonPoint {
            x: (p.x as f32 / width_f).clamp(0.0, 1.0),
            y: (p.y as f32 / height_f).clamp(0.0, 1.0),
        });
    }

    Some(points)
}

#[cfg(target_os = "linux")]
pub(crate) fn rectify_snapshot_linux(
    frame_data_url: &str,
    polygon: &[PolygonPoint],
    post_process: &str,
    denoise_strength: &str,
) -> Result<String, String> {
    let frame_bytes = decode_data_url_base64(frame_data_url)?;
    if !looks_like_jpeg(&frame_bytes) {
        return Err("当前矫正仅支持 JPEG 截图。".into());
    }

    let input = Vector::<u8>::from_slice(&frame_bytes);
    let frame = imgcodecs::imdecode(&input, imgcodecs::IMREAD_COLOR)
        .map_err(|e| format!("解码截图失败: {e}"))?;
    if frame.empty() {
        return Err("截图为空，无法矫正。".into());
    }

    let width = frame.cols();
    let height = frame.rows();
    let quad = normalized_polygon_to_quad(polygon, width, height)?;

    let target_width = paper_target_width(&quad);
    let target_height = paper_target_height(&quad);

    let mut src = Vector::<Point2f>::new();
    for p in quad {
        src.push(p);
    }

    let mut dst = Vector::<Point2f>::new();
    dst.push(Point2f::new(0.0, 0.0));
    dst.push(Point2f::new((target_width - 1) as f32, 0.0));
    dst.push(Point2f::new(
        (target_width - 1) as f32,
        (target_height - 1) as f32,
    ));
    dst.push(Point2f::new(0.0, (target_height - 1) as f32));

    let matrix = imgproc::get_perspective_transform(&src, &dst, core::DECOMP_LU)
        .map_err(|e| format!("计算透视矩阵失败: {e}"))?;

    let mut warped = Mat::default();
    imgproc::warp_perspective(
        &frame,
        &mut warped,
        &matrix,
        Size::new(target_width, target_height),
        imgproc::INTER_LINEAR,
        core::BORDER_REPLICATE,
        Scalar::all(0.0),
    )
    .map_err(|e| format!("透视变换失败: {e}"))?;

    let use_sharpen = !post_process.eq_ignore_ascii_case("none");
    let output_mat = if use_sharpen {
        post_process_sharpen_text_bw(&warped, denoise_strength)?
    } else {
        warped
    };

    let mut output = Vector::<u8>::new();
    let mut params = Vector::<i32>::new();
    let (ext, mime) = if use_sharpen {
        params.push(imgcodecs::IMWRITE_PNG_COMPRESSION);
        params.push(3);
        (".png", "image/png")
    } else {
        params.push(imgcodecs::IMWRITE_JPEG_QUALITY);
        params.push(95);
        (".jpg", "image/jpeg")
    };

    imgcodecs::imencode(ext, &output_mat, &mut output, &params)
        .map_err(|e| format!("编码矫正结果失败: {e}"))?;

    Ok(format!(
        "data:{mime};base64,{}",
        STANDARD.encode(output.as_slice())
    ))
}

#[cfg(target_os = "linux")]
#[derive(Clone, Copy, Debug)]
struct DenoiseProfile {
    color_h_primary: f32,
    color_h_color_primary: f32,
    color_secondary: Option<(f32, f32)>,
    gray_h: f32,
    template_window: i32,
    search_window: i32,
}

#[cfg(target_os = "linux")]
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

#[cfg(target_os = "linux")]
fn post_process_sharpen_text_bw(warped: &Mat, denoise_strength: &str) -> Result<Mat, String> {
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

#[cfg(target_os = "linux")]
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

#[cfg(target_os = "linux")]
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

#[cfg(target_os = "linux")]
fn decode_data_url_base64(data_url: &str) -> Result<Vec<u8>, String> {
    let base64_part = data_url
        .split_once(',')
        .map(|(_, b64)| b64)
        .ok_or_else(|| "无效的截图数据格式。".to_string())?;

    STANDARD
        .decode(base64_part)
        .map_err(|e| format!("解析截图 base64 失败: {e}"))
}

#[cfg(target_os = "linux")]
fn normalized_polygon_to_quad(
    polygon: &[PolygonPoint],
    width: i32,
    height: i32,
) -> Result<[Point2f; 4], String> {
    if polygon.len() < 4 {
        return Err("矫正至少需要 4 个顶点。".into());
    }

    let mut contour = Vector::<Point>::new();
    for p in polygon {
        let x = (p.x.clamp(0.0, 1.0) * (width - 1) as f32).round() as i32;
        let y = (p.y.clamp(0.0, 1.0) * (height - 1) as f32).round() as i32;
        contour.push(Point::new(x, y));
    }

    let mut hull = Vector::<Point>::new();
    imgproc::convex_hull(&contour, &mut hull, false, true)
        .map_err(|e| format!("计算凸包失败: {e}"))?;

    let mut selected: Option<[Point2f; 4]> = None;
    let perimeter = imgproc::arc_length(&hull, true).unwrap_or(0.0);
    for epsilon_factor in [0.01_f64, 0.015, 0.02, 0.03, 0.04] {
        let mut approx = Vector::<Point>::new();
        if imgproc::approx_poly_dp(&hull, &mut approx, perimeter * epsilon_factor, true).is_err() {
            continue;
        }
        if approx.len() != 4 || !imgproc::is_contour_convex(&approx).unwrap_or(false) {
            continue;
        }

        let mut corners = [Point2f::new(0.0, 0.0); 4];
        let mut ok = true;
        for (i, corner) in corners.iter_mut().enumerate() {
            match approx.get(i) {
                Ok(pt) => *corner = Point2f::new(pt.x as f32, pt.y as f32),
                Err(_) => {
                    ok = false;
                    break;
                }
            }
        }
        if ok {
            selected = Some(corners);
            break;
        }
    }

    if selected.is_none() {
        let rect =
            imgproc::min_area_rect(&hull).map_err(|e| format!("计算最小外接矩形失败: {e}"))?;
        let mut pts = [Point2f::new(0.0, 0.0); 4];
        rect.points(&mut pts)
            .map_err(|e| format!("获取矩形角点失败: {e}"))?;
        selected = Some(pts);
    }

    let ordered = order_quad_points_f32(
        selected.ok_or_else(|| "无法从当前顶点拟合有效四边形。".to_string())?,
    );

    if polygon_area_abs_f32(&ordered) < 200.0 {
        return Err("选区面积过小，无法矫正。".into());
    }

    Ok(ordered)
}

#[cfg(target_os = "linux")]
fn order_quad_points_f32(points: [Point2f; 4]) -> [Point2f; 4] {
    let mut tl = points[0];
    let mut tr = points[0];
    let mut br = points[0];
    let mut bl = points[0];
    let mut min_sum = f32::MAX;
    let mut max_sum = f32::MIN;
    let mut min_diff = f32::MAX;
    let mut max_diff = f32::MIN;

    for p in points {
        let sum = p.x + p.y;
        let diff = p.x - p.y;
        if sum < min_sum {
            min_sum = sum;
            tl = p;
        }
        if sum > max_sum {
            max_sum = sum;
            br = p;
        }
        if diff > max_diff {
            max_diff = diff;
            tr = p;
        }
        if diff < min_diff {
            min_diff = diff;
            bl = p;
        }
    }

    [tl, tr, br, bl]
}

#[cfg(target_os = "linux")]
fn polygon_area_abs_f32(quad: &[Point2f; 4]) -> f64 {
    let mut area = 0.0_f64;
    for i in 0..4 {
        let p = quad[i];
        let q = quad[(i + 1) % 4];
        area += f64::from(p.x) * f64::from(q.y) - f64::from(p.y) * f64::from(q.x);
    }
    area.abs() * 0.5
}

#[cfg(target_os = "linux")]
fn edge_length_f32(a: Point2f, b: Point2f) -> f32 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    (dx * dx + dy * dy).sqrt()
}

#[cfg(target_os = "linux")]
fn paper_target_width(quad: &[Point2f; 4]) -> i32 {
    let w_top = edge_length_f32(quad[0], quad[1]);
    let w_bottom = edge_length_f32(quad[3], quad[2]);
    w_top.max(w_bottom).round().clamp(64.0, 4096.0) as i32
}

#[cfg(target_os = "linux")]
fn paper_target_height(quad: &[Point2f; 4]) -> i32 {
    let h_left = edge_length_f32(quad[0], quad[3]);
    let h_right = edge_length_f32(quad[1], quad[2]);
    h_left.max(h_right).round().clamp(64.0, 4096.0) as i32
}

#[cfg(target_os = "linux")]
fn detect_document_quad_opencv(color: &Mat) -> Option<[PointI; 4]> {
    let width = color.cols();
    let height = color.rows();
    if width < 20 || height < 20 {
        return None;
    }

    let image_area = f64::from(width * height);
    let mut best: Option<([PointI; 4], f64)> = None;

    let mut reduced = Mat::default();
    let mut smoothed = Mat::default();
    let reduced_size = Size::new((width / 2).max(1), (height / 2).max(1));
    imgproc::pyr_down(color, &mut reduced, reduced_size, core::BORDER_DEFAULT).ok()?;
    imgproc::pyr_up(
        &reduced,
        &mut smoothed,
        Size::new(width, height),
        core::BORDER_DEFAULT,
    )
    .ok()?;
    let source = if smoothed.empty() {
        color.try_clone().ok()?
    } else {
        smoothed
    };

    if let Some((quad, score)) =
        find_best_quad_from_squares_like(&source, width, height, image_area)
    {
        best = Some((quad, score));
    }

    if let Some((quad, score)) = find_best_quad_from_hsv_white(&source, width, height, image_area) {
        best = pick_better(best, (quad, score));
    }

    let mut gray = Mat::default();
    imgproc::cvt_color(&source, &mut gray, imgproc::COLOR_BGR2GRAY, 0).ok()?;

    let mut normalized = Mat::default();
    imgproc::equalize_hist(&gray, &mut normalized).ok()?;

    let mut blurred = Mat::default();
    imgproc::gaussian_blur(
        &normalized,
        &mut blurred,
        Size::new(5, 5),
        0.0,
        0.0,
        core::BORDER_DEFAULT,
    )
    .ok()?;

    let mut edges = Mat::default();
    imgproc::canny(&blurred, &mut edges, 35.0, 120.0, 3, false).ok()?;
    if let Some((quad, score)) =
        find_best_quad_from_binary(&edges, width, height, image_area, 1.0, 1)
    {
        best = pick_better(best, (quad, score));
    }

    let mut adaptive_inv = Mat::default();
    imgproc::adaptive_threshold(
        &blurred,
        &mut adaptive_inv,
        255.0,
        imgproc::ADAPTIVE_THRESH_GAUSSIAN_C,
        imgproc::THRESH_BINARY_INV,
        31,
        7.0,
    )
    .ok()?;
    if let Some((quad, score)) =
        find_best_quad_from_binary(&adaptive_inv, width, height, image_area, 1.02, 2)
    {
        best = pick_better(best, (quad, score));
    }

    let mut otsu_inv = Mat::default();
    imgproc::threshold(
        &blurred,
        &mut otsu_inv,
        0.0,
        255.0,
        imgproc::THRESH_BINARY_INV | imgproc::THRESH_OTSU,
    )
    .ok()?;
    if let Some((quad, score)) =
        find_best_quad_from_binary(&otsu_inv, width, height, image_area, 0.98, 1)
    {
        best = pick_better(best, (quad, score));
    }

    best.map(|(quad, _)| quad)
}

#[cfg(target_os = "linux")]
fn find_best_quad_from_squares_like(
    color: &Mat,
    width: i32,
    height: i32,
    image_area: f64,
) -> Option<([PointI; 4], f64)> {
    let mut channels = Vector::<Mat>::new();
    core::split(color, &mut channels).ok()?;

    let mut best: Option<([PointI; 4], f64)> = None;
    let levels = 8_i32;

    for ci in 0..channels.len() {
        let channel = channels.get(ci).ok()?;

        for level in 0..levels {
            let mut binary = Mat::default();
            if level == 0 {
                imgproc::canny(&channel, &mut binary, 0.0, 60.0, 5, false).ok()?;
                let mut dilated = Mat::default();
                imgproc::dilate(
                    &binary,
                    &mut dilated,
                    &Mat::default(),
                    Point::new(-1, -1),
                    1,
                    core::BORDER_CONSTANT,
                    Scalar::all(0.0),
                )
                .ok()?;
                binary = dilated;
            } else {
                let threshold = f64::from((level + 1) * 255 / levels);
                imgproc::threshold(
                    &channel,
                    &mut binary,
                    threshold,
                    255.0,
                    imgproc::THRESH_BINARY,
                )
                .ok()?;
            }

            if let Some((quad, score)) =
                find_best_quad_from_binary(&binary, width, height, image_area, 1.03, 1)
            {
                best = pick_better(best, (quad, score));
            }
        }
    }

    best
}

#[cfg(target_os = "linux")]
fn find_best_quad_from_hsv_white(
    color: &Mat,
    width: i32,
    height: i32,
    image_area: f64,
) -> Option<([PointI; 4], f64)> {
    let mut hsv = Mat::default();
    imgproc::cvt_color(color, &mut hsv, imgproc::COLOR_BGR2HSV, 0).ok()?;

    let mut white_mask = Mat::default();
    core::in_range(
        &hsv,
        &Scalar::new(0.0, 0.0, 105.0, 0.0),
        &Scalar::new(180.0, 100.0, 255.0, 0.0),
        &mut white_mask,
    )
    .ok()?;

    find_best_quad_from_binary(&white_mask, width, height, image_area, 1.08, 2)
}

#[cfg(target_os = "linux")]
fn pick_better(
    current: Option<([PointI; 4], f64)>,
    candidate: ([PointI; 4], f64),
) -> Option<([PointI; 4], f64)> {
    match current {
        None => Some(candidate),
        Some((best_quad, best_score)) => {
            if candidate.1 > best_score {
                Some(candidate)
            } else {
                Some((best_quad, best_score))
            }
        }
    }
}

#[cfg(target_os = "linux")]
fn find_best_quad_from_binary(
    binary: &Mat,
    width: i32,
    height: i32,
    image_area: f64,
    pipeline_weight: f64,
    close_iterations: i32,
) -> Option<([PointI; 4], f64)> {
    let iterations = close_iterations.clamp(0, 3);
    let closed = if iterations > 0 {
        let kernel = imgproc::get_structuring_element(
            imgproc::MORPH_RECT,
            Size::new(5, 5),
            Point::new(-1, -1),
        )
        .ok()?;

        let mut out = Mat::default();
        imgproc::morphology_ex(
            binary,
            &mut out,
            imgproc::MORPH_CLOSE,
            &kernel,
            Point::new(-1, -1),
            iterations,
            core::BORDER_CONSTANT,
            Scalar::all(0.0),
        )
        .ok()?;
        out
    } else {
        binary.try_clone().ok()?
    };

    let mut contours = Vector::<Vector<Point>>::new();
    imgproc::find_contours(
        &closed,
        &mut contours,
        imgproc::RETR_EXTERNAL,
        imgproc::CHAIN_APPROX_SIMPLE,
        Point::new(0, 0),
    )
    .ok()?;

    let mut best: Option<([PointI; 4], f64)> = None;

    for i in 0..contours.len() {
        let contour = match contours.get(i) {
            Ok(c) => c,
            Err(_) => continue,
        };

        if contour.len() < 4 {
            continue;
        }

        let contour_area_value = match imgproc::contour_area(&contour, false) {
            Ok(a) => a.abs(),
            Err(_) => continue,
        };

        if contour_area_value < image_area * 0.015 || contour_area_value > image_area * 0.995 {
            continue;
        }

        let perimeter = match imgproc::arc_length(&contour, true) {
            Ok(p) => p,
            Err(_) => continue,
        };

        if perimeter < f64::from((width + height).max(1)) * 0.06 {
            continue;
        }

        let mut selected: Option<([PointI; 4], bool)> = None;

        for epsilon_factor in [0.01_f64, 0.015, 0.02, 0.03, 0.04, 0.05] {
            let mut approx = Vector::<Point>::new();
            if imgproc::approx_poly_dp(&contour, &mut approx, perimeter * epsilon_factor, true)
                .is_err()
            {
                continue;
            }
            if approx.len() != 4 {
                continue;
            }
            if !imgproc::is_contour_convex(&approx).unwrap_or(false) {
                continue;
            }
            if let Some(quad) = contour_vector_to_quad(&approx) {
                selected = Some((quad, true));
                break;
            }
        }

        if selected.is_none() {
            let rect = match imgproc::min_area_rect(&contour) {
                Ok(r) => r,
                Err(_) => continue,
            };
            let mut pts = [Point2f::new(0.0, 0.0); 4];
            if rect.points(&mut pts).is_err() {
                continue;
            }
            let quad = order_quad_points([
                PointI {
                    x: pts[0].x.round() as i32,
                    y: pts[0].y.round() as i32,
                },
                PointI {
                    x: pts[1].x.round() as i32,
                    y: pts[1].y.round() as i32,
                },
                PointI {
                    x: pts[2].x.round() as i32,
                    y: pts[2].y.round() as i32,
                },
                PointI {
                    x: pts[3].x.round() as i32,
                    y: pts[3].y.round() as i32,
                },
            ]);
            selected = Some((quad, false));
        }

        let Some((quad, from_poly_approx)) = selected else {
            continue;
        };

        if !is_document_like_quad(&quad, width, height, image_area) {
            continue;
        }

        let quad_area = polygon_area_abs(&quad);
        if quad_area <= 1.0 {
            continue;
        }

        let fill_ratio = (contour_area_value / quad_area).clamp(0.35, 1.25);
        let rectangularity = (1.0 - max_corner_cosine(&quad)).clamp(0.15, 1.0);
        let aspect_ratio = quad_aspect_ratio(&quad);
        let paper_ratio = 2.0_f64.sqrt();
        let aspect_delta = (aspect_ratio - paper_ratio).abs();
        let aspect_score = if aspect_delta < 0.7 {
            1.0 - aspect_delta * 0.28
        } else {
            0.78
        };

        let center = quad_center(&quad);
        let nx = (center.0 / f64::from(width) - 0.5) * 2.0;
        let ny = (center.1 / f64::from(height) - 0.5) * 2.0;
        let center_dist = (nx * nx + ny * ny).sqrt().min(1.4);
        let center_score = (1.12 - center_dist * 0.24).clamp(0.68, 1.12);

        let approx_bonus = if from_poly_approx {
            1.0
        } else {
            0.88 * fill_ratio.max(0.5)
        };
        let score = quad_area
            * fill_ratio
            * rectangularity
            * aspect_score
            * center_score
            * approx_bonus
            * pipeline_weight;

        best = pick_better(best, (quad, score));
    }

    best
}

#[cfg(target_os = "linux")]
fn contour_vector_to_quad(points: &Vector<Point>) -> Option<[PointI; 4]> {
    if points.len() != 4 {
        return None;
    }

    let mut arr = [PointI { x: 0, y: 0 }; 4];
    for (i, slot) in arr.iter_mut().enumerate() {
        let p = points.get(i).ok()?;
        *slot = PointI { x: p.x, y: p.y };
    }

    let ordered = order_quad_points(arr);
    if unique_points_count(&ordered) < 4 {
        None
    } else {
        Some(ordered)
    }
}

#[cfg(target_os = "linux")]
fn order_quad_points(points: [PointI; 4]) -> [PointI; 4] {
    let mut pts = points;
    pts.sort_by(|a, b| a.y.cmp(&b.y).then_with(|| a.x.cmp(&b.x)));

    let mut top = [pts[0], pts[1]];
    top.sort_by(|a, b| a.x.cmp(&b.x));

    let mut bottom = [pts[2], pts[3]];
    bottom.sort_by(|a, b| a.x.cmp(&b.x));

    [top[0], top[1], bottom[1], bottom[0]]
}

#[cfg(target_os = "linux")]
fn unique_points_count(points: &[PointI; 4]) -> usize {
    let mut unique = Vec::new();
    for &p in points {
        if !unique.contains(&p) {
            unique.push(p);
        }
    }
    unique.len()
}

#[cfg(target_os = "linux")]
fn is_document_like_quad(quad: &[PointI; 4], width: i32, height: i32, image_area: f64) -> bool {
    if !is_convex_quad(quad) {
        return false;
    }

    let area = polygon_area_abs(quad);
    if area < image_area * 0.08 || area > image_area * 0.995 {
        return false;
    }

    if shortest_edge(quad) < 24.0 {
        return false;
    }

    let edge_ratio = edge_balance_ratio(quad);
    if !(0.2..=6.5).contains(&edge_ratio) {
        return false;
    }

    if max_corner_cosine(quad) > 0.72 {
        return false;
    }

    let border_margin = ((width.min(height) as f32) * 0.01).round().max(2.0) as i32;
    let on_border = quad
        .iter()
        .filter(|p| {
            p.x <= border_margin
                || p.y <= border_margin
                || p.x >= width - border_margin - 1
                || p.y >= height - border_margin - 1
        })
        .count();

    on_border < 4
}

#[cfg(target_os = "linux")]
fn polygon_area_abs(quad: &[PointI; 4]) -> f64 {
    let mut area = 0_f64;
    for i in 0..4 {
        let p = quad[i];
        let q = quad[(i + 1) % 4];
        area += f64::from(p.x) * f64::from(q.y) - f64::from(p.y) * f64::from(q.x);
    }
    area.abs() * 0.5
}

#[cfg(target_os = "linux")]
fn is_convex_quad(quad: &[PointI; 4]) -> bool {
    let mut positive = 0;
    let mut negative = 0;

    for i in 0..4 {
        let o = quad[i];
        let a = quad[(i + 1) % 4];
        let b = quad[(i + 2) % 4];
        let c = cross(o, a, b);
        if c > 0 {
            positive += 1;
        } else if c < 0 {
            negative += 1;
        }
    }

    positive == 4 || negative == 4
}

#[cfg(target_os = "linux")]
fn shortest_edge(quad: &[PointI; 4]) -> f64 {
    let mut best = f64::MAX;
    for i in 0..4 {
        let a = quad[i];
        let b = quad[(i + 1) % 4];
        let dx = f64::from(a.x - b.x);
        let dy = f64::from(a.y - b.y);
        best = best.min((dx * dx + dy * dy).sqrt());
    }
    best
}

#[cfg(target_os = "linux")]
fn edge_balance_ratio(quad: &[PointI; 4]) -> f64 {
    let mut min_edge = f64::INFINITY;
    let mut max_edge = 0.0_f64;

    for i in 0..4 {
        let a = quad[i];
        let b = quad[(i + 1) % 4];
        let dx = f64::from(a.x - b.x);
        let dy = f64::from(a.y - b.y);
        let len = (dx * dx + dy * dy).sqrt();
        min_edge = min_edge.min(len);
        max_edge = max_edge.max(len);
    }

    if min_edge <= 1.0 {
        999.0
    } else {
        max_edge / min_edge
    }
}

#[cfg(target_os = "linux")]
fn edge_length_i(a: PointI, b: PointI) -> f64 {
    let dx = f64::from(a.x - b.x);
    let dy = f64::from(a.y - b.y);
    (dx * dx + dy * dy).sqrt()
}

#[cfg(target_os = "linux")]
fn quad_aspect_ratio(quad: &[PointI; 4]) -> f64 {
    let width = (edge_length_i(quad[0], quad[1]) + edge_length_i(quad[2], quad[3])) * 0.5;
    let height = (edge_length_i(quad[1], quad[2]) + edge_length_i(quad[3], quad[0])) * 0.5;
    let min_side = width.min(height).max(1.0);
    let max_side = width.max(height);
    max_side / min_side
}

#[cfg(target_os = "linux")]
fn quad_center(quad: &[PointI; 4]) -> (f64, f64) {
    let mut cx = 0.0;
    let mut cy = 0.0;
    for p in quad {
        cx += f64::from(p.x);
        cy += f64::from(p.y);
    }
    (cx / 4.0, cy / 4.0)
}

#[cfg(target_os = "linux")]
fn max_corner_cosine(quad: &[PointI; 4]) -> f64 {
    let mut worst = 0.0_f64;

    for i in 0..4 {
        let prev = quad[(i + 3) % 4];
        let cur = quad[i];
        let next = quad[(i + 1) % 4];

        let v1x = f64::from(prev.x - cur.x);
        let v1y = f64::from(prev.y - cur.y);
        let v2x = f64::from(next.x - cur.x);
        let v2y = f64::from(next.y - cur.y);

        let d1 = (v1x * v1x + v1y * v1y).sqrt().max(1.0);
        let d2 = (v2x * v2x + v2y * v2y).sqrt().max(1.0);
        let cosine = ((v1x * v2x + v1y * v2y) / (d1 * d2)).abs();
        worst = worst.max(cosine);
    }

    worst
}

#[cfg(target_os = "linux")]
fn cross(o: PointI, a: PointI, b: PointI) -> i64 {
    i64::from(a.x - o.x) * i64::from(b.y - o.y) - i64::from(a.y - o.y) * i64::from(b.x - o.x)
}
