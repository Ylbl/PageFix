pub(crate) fn push_interval_unique(intervals: &mut Vec<(u32, u32)>, interval: (u32, u32)) {
    if interval.0 == 0 || interval.1 == 0 {
        return;
    }
    if !intervals.contains(&interval) {
        intervals.push(interval);
    }
}

pub(crate) fn interval_to_fps(interval: (u32, u32)) -> u32 {
    if interval.0 == 0 {
        return 0;
    }
    interval.1 / interval.0
}

pub(crate) fn fps_distance(a: (u32, u32), b: (u32, u32)) -> f64 {
    let afps = if a.0 == 0 {
        0.0
    } else {
        a.1 as f64 / a.0 as f64
    };
    let bfps = if b.0 == 0 {
        0.0
    } else {
        b.1 as f64 / b.0 as f64
    };
    (afps - bfps).abs()
}
