mod correction;
mod detection;
mod geometry;
mod postprocess;

pub(crate) use correction::rectify_snapshot_linux;
pub(crate) use detection::{detect_document_fast, looks_like_jpeg};
