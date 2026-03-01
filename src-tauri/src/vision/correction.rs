use base64::{engine::general_purpose::STANDARD, Engine as _};
use opencv::{
    core::{self, Mat, Point2f, Scalar, Size, Vector},
    imgcodecs, imgproc,
    prelude::*,
};

use crate::models::PolygonPoint;
use super::geometry::{normalized_polygon_to_quad, paper_target_height, paper_target_width};
use super::postprocess::post_process_sharpen_text_bw;

pub(crate) fn rectify_snapshot_linux(
    frame_data_url: &str,
    polygon: &[PolygonPoint],
    post_process: &str,
    denoise_strength: &str,
) -> Result<String, String> {
    let frame_bytes = decode_data_url_base64(frame_data_url)?;
    if !super::detection::looks_like_jpeg(&frame_bytes) {
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

fn decode_data_url_base64(data_url: &str) -> Result<Vec<u8>, String> {
    let base64_part = data_url
        .split_once(',')
        .map(|(_, b64)| b64)
        .ok_or_else(|| "无效的截图数据格式。".to_string())?;
    STANDARD
        .decode(base64_part)
        .map_err(|e| format!("解析截图 base64 失败: {e}"))
}
