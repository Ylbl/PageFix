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
