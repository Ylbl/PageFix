use opencv::{
    core::{self, Mat, Point, Point2f, Scalar, Size, Vector},
    imgcodecs, imgproc,
    prelude::*,
};

use crate::models::PolygonPoint;
use super::geometry::{
    contour_vector_to_quad, is_document_like_quad, max_corner_cosine, order_quad_points,
    pick_better, polygon_area_abs, quad_aspect_ratio, quad_center, PointI,
};

pub(crate) fn looks_like_jpeg(bytes: &[u8]) -> bool {
    bytes.len() >= 4
        && bytes[0] == 0xFF
        && bytes[1] == 0xD8
        && bytes[bytes.len() - 2] == 0xFF
        && bytes[bytes.len() - 1] == 0xD9
}

/// Fast single-pass document detection for real-time preview.
///
/// Pipeline (optimized for speed, targets A4 paper):
///   1. Decode JPEG → downscale to ≤480p
///   2. Grayscale → GaussianBlur(5×5)
///   3. Canny edge detection (dual threshold)
///   4. Dilate to close small gaps in edges
///   5. findContours → sort by area → approxPolyDP for 4-point quad
///   6. Validate: convex, reasonable area, near-rectangular angles
///
/// Returns normalized [0,1] polygon or None.
pub(crate) fn detect_document_fast(
    jpeg_bytes: &[u8],
    canny_weight: f64,
    hsv_weight: f64,
) -> Option<Vec<PolygonPoint>> {
    let input = Vector::<u8>::from_slice(jpeg_bytes);
    let color_full = imgcodecs::imdecode(&input, imgcodecs::IMREAD_COLOR).ok()?;
    if color_full.empty() {
        return None;
    }
    let full_w = color_full.cols();
    let full_h = color_full.rows();
    if full_w < 20 || full_h < 20 {
        return None;
    }
    // Downscale to ≤480p for speed (edge detection and HSV).
    let max_dim = 480_i32;
    let max_side = full_w.max(full_h);
    let color = if max_side > max_dim {
        let ratio = f64::from(max_dim) / f64::from(max_side);
        let sw = (f64::from(full_w) * ratio).round() as i32;
        let sh = (f64::from(full_h) * ratio).round() as i32;
        let mut resized = Mat::default();
        imgproc::resize(
            &color_full,
            &mut resized,
            Size::new(sw.max(1), sh.max(1)),
            0.0,
            0.0,
            imgproc::INTER_AREA,
        )
        .ok()?;
        resized
    } else {
        color_full.try_clone().ok()?
    };
    let w = color.cols();
    let h = color.rows();
    let image_area = f64::from(w * h);

    // Grayscale + blur for edge detection.
    let mut gray = Mat::default();
    imgproc::cvt_color(&color, &mut gray, imgproc::COLOR_BGR2GRAY, 0).ok()?;
    let mut blurred = Mat::default();
    imgproc::gaussian_blur(
        &gray,
        &mut blurred,
        Size::new(5, 5),
        0.0,
        0.0,
        core::BORDER_DEFAULT,
    )
    .ok()?;

    // Try two Canny threshold pairs: one for high-contrast edges, one for softer edges.
    let mut best: Option<([PointI; 4], f64)> = None;
    for (lo, hi) in [(50.0, 150.0), (30.0, 90.0)] {
        let mut edges = Mat::default();
        imgproc::canny(&blurred, &mut edges, lo, hi, 3, false).ok()?;
        // Dilate to bridge small gaps in document edges.
        let mut dilated = Mat::default();
        imgproc::dilate(
            &edges,
            &mut dilated,
            &Mat::default(),
            Point::new(-1, -1),
            2,
            core::BORDER_CONSTANT,
            Scalar::all(0.0),
        )
        .ok()?;
        if let Some(candidate) = find_best_quad_from_binary(&dilated, w, h, image_area, canny_weight, 0) {
            best = pick_better(best, candidate);
        }
    }

    // Also try HSV white-paper mask (very effective for white A4 on colored backgrounds).
    if let Some(candidate) = find_best_quad_from_hsv_white(&color, w, h, image_area, hsv_weight) {
        best = pick_better(best, candidate);
    }

    let quad = best?.0;
    let wf = w as f32;
    let hf = h as f32;
    let mut points = Vec::with_capacity(4);
    for p in quad {
        points.push(PolygonPoint {
            x: (p.x as f32 / wf).clamp(0.0, 1.0),
            y: (p.y as f32 / hf).clamp(0.0, 1.0),
        });
    }
    Some(points)
}

/// Detect dark text/ink regions and compute the minimum bounding rectangle.
///
/// This is the fallback for white-paper-on-white-background where edge detection
/// fails. The pipeline:
///   1. OTSU threshold (inverted) to isolate dark text as white pixels
///   2. Moderate dilation to merge characters → text blocks
///   3. Collect all qualifying contour points into one set
///   4. minAreaRect on the combined points → document bounding quad

fn find_best_quad_from_hsv_white(
    color: &Mat,
    width: i32,
    height: i32,
    image_area: f64,
    weight: f64,
) -> Option<([PointI; 4], f64)> {
    let mut hsv = Mat::default();
    imgproc::cvt_color(color, &mut hsv, imgproc::COLOR_BGR2HSV, 0).ok()?;

    // Extract V channel (brightness).
    let mut channels = Vector::<Mat>::new();
    core::split(&hsv, &mut channels).ok()?;
    let v_channel = channels.get(2).ok()?;

    // Find the brightest regions (top 20% brightness).
    let mut bright_mask = Mat::default();
    imgproc::threshold(
        &v_channel,
        &mut bright_mask,
        0.0,
        255.0,
        imgproc::THRESH_BINARY | imgproc::THRESH_OTSU,
    )
    .ok()?;

    // Apply morphological close to fill gaps, then open to remove noise.
    let kernel = imgproc::get_structuring_element(
        imgproc::MORPH_RECT,
        Size::new(5, 5),
        Point::new(-1, -1),
    )
    .ok()?;
    let mut closed = Mat::default();
    imgproc::morphology_ex(
        &bright_mask,
        &mut closed,
        imgproc::MORPH_CLOSE,
        &kernel,
        Point::new(-1, -1),
        2,
        core::BORDER_CONSTANT,
        Scalar::all(0.0),
    )
    .ok()?;

    find_best_quad_from_binary(&closed, width, height, image_area, weight, 1)
}

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
