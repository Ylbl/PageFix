use opencv::{
    core::{Point, Point2f, Vector},
    imgproc,
};

use crate::models::PolygonPoint;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct PointI {
    pub(super) x: i32,
    pub(super) y: i32,
}

pub(super) fn order_quad_points(points: [PointI; 4]) -> [PointI; 4] {
    let mut pts = points;
    pts.sort_by(|a, b| a.y.cmp(&b.y).then_with(|| a.x.cmp(&b.x)));
    let mut top = [pts[0], pts[1]];
    top.sort_by(|a, b| a.x.cmp(&b.x));
    let mut bottom = [pts[2], pts[3]];
    bottom.sort_by(|a, b| a.x.cmp(&b.x));
    [top[0], top[1], bottom[1], bottom[0]]
}

pub(super) fn unique_points_count(points: &[PointI; 4]) -> usize {
    let mut unique = Vec::new();
    for &p in points {
        if !unique.contains(&p) {
            unique.push(p);
        }
    }
    unique.len()
}

pub(super) fn contour_vector_to_quad(points: &Vector<Point>) -> Option<[PointI; 4]> {
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

pub(super) fn is_convex_quad(quad: &[PointI; 4]) -> bool {
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

fn cross(o: PointI, a: PointI, b: PointI) -> i64 {
    i64::from(a.x - o.x) * i64::from(b.y - o.y) - i64::from(a.y - o.y) * i64::from(b.x - o.x)
}

pub(super) fn polygon_area_abs(quad: &[PointI; 4]) -> f64 {
    let mut area = 0_f64;
    for i in 0..4 {
        let p = quad[i];
        let q = quad[(i + 1) % 4];
        area += f64::from(p.x) * f64::from(q.y) - f64::from(p.y) * f64::from(q.x);
    }
    area.abs() * 0.5
}

pub(super) fn shortest_edge(quad: &[PointI; 4]) -> f64 {
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

pub(super) fn edge_balance_ratio(quad: &[PointI; 4]) -> f64 {
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

fn edge_length_i(a: PointI, b: PointI) -> f64 {
    let dx = f64::from(a.x - b.x);
    let dy = f64::from(a.y - b.y);
    (dx * dx + dy * dy).sqrt()
}

pub(super) fn quad_aspect_ratio(quad: &[PointI; 4]) -> f64 {
    let width = (edge_length_i(quad[0], quad[1]) + edge_length_i(quad[2], quad[3])) * 0.5;
    let height = (edge_length_i(quad[1], quad[2]) + edge_length_i(quad[3], quad[0])) * 0.5;
    let min_side = width.min(height).max(1.0);
    let max_side = width.max(height);
    max_side / min_side
}

pub(super) fn quad_center(quad: &[PointI; 4]) -> (f64, f64) {
    let mut cx = 0.0;
    let mut cy = 0.0;
    for p in quad {
        cx += f64::from(p.x);
        cy += f64::from(p.y);
    }
    (cx / 4.0, cy / 4.0)
}

pub(super) fn max_corner_cosine(quad: &[PointI; 4]) -> f64 {
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

pub(super) fn is_document_like_quad(
    quad: &[PointI; 4],
    width: i32,
    height: i32,
    image_area: f64,
) -> bool {
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

pub(super) fn pick_better(
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

pub(super) fn polygon_area_abs_f32(quad: &[Point2f; 4]) -> f64 {
    let mut area = 0.0_f64;
    for i in 0..4 {
        let p = quad[i];
        let q = quad[(i + 1) % 4];
        area += f64::from(p.x) * f64::from(q.y) - f64::from(p.y) * f64::from(q.x);
    }
    area.abs() * 0.5
}

fn edge_length_f32(a: Point2f, b: Point2f) -> f32 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    (dx * dx + dy * dy).sqrt()
}

pub(super) fn paper_target_width(quad: &[Point2f; 4]) -> i32 {
    let w_top = edge_length_f32(quad[0], quad[1]);
    let w_bottom = edge_length_f32(quad[3], quad[2]);
    w_top.max(w_bottom).round().clamp(64.0, 4096.0) as i32
}

pub(super) fn paper_target_height(quad: &[Point2f; 4]) -> i32 {
    let h_left = edge_length_f32(quad[0], quad[3]);
    let h_right = edge_length_f32(quad[1], quad[2]);
    h_left.max(h_right).round().clamp(64.0, 4096.0) as i32
}

pub(super) fn order_quad_points_f32(points: [Point2f; 4]) -> [Point2f; 4] {
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

pub(super) fn normalized_polygon_to_quad(
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
