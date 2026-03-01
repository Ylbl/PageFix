pub(crate) fn tune_camera_controls_linux(camera: &rscam::Camera) {
    use rscam::{consts, Control};

    for control in camera.controls().flatten() {
        if control.flags & (consts::FLAG_DISABLED | consts::FLAG_READ_ONLY | consts::FLAG_INACTIVE)
            != 0
        {
            continue;
        }
        let name = control.name.to_ascii_lowercase();
        if is_quality_control_name(&name) {
            apply_control_target(camera, &control, true);
            continue;
        }
        if is_noise_reduction_control_name(&name) {
            apply_control_target(camera, &control, false);
        }
    }

    fn apply_control_target(camera: &rscam::Camera, control: &Control, quality_mode: bool) {
        use rscam::CtrlData;
        match &control.data {
            CtrlData::Boolean { .. } => {
                let _ = camera.set_control(control.id, &true);
            }
            CtrlData::Integer {
                minimum,
                maximum,
                step,
                ..
            } => {
                let target = if quality_mode {
                    *maximum
                } else {
                    *minimum + (((*maximum - *minimum) as f32 * 0.85) as i32)
                };
                let quantized = quantize_int_to_step(target, *minimum, *maximum, *step);
                let _ = camera.set_control(control.id, &quantized);
            }
            CtrlData::Integer64 {
                minimum,
                maximum,
                step,
                ..
            } => {
                let target = if quality_mode {
                    *maximum
                } else {
                    *minimum + (((*maximum - *minimum) as f64 * 0.85) as i64)
                };
                let quantized = quantize_i64_to_step(target, *minimum, *maximum, *step);
                let _ = camera.set_control(control.id, &quantized);
            }
            CtrlData::Menu { items, .. } => {
                if let Some(index) = pick_menu_index(items, quality_mode) {
                    let _ = camera.set_control(control.id, &index);
                }
            }
            CtrlData::IntegerMenu { items, .. } => {
                let target = if quality_mode {
                    items.iter().max_by_key(|it| it.value)
                } else {
                    items.iter().max_by_key(|it| it.value)
                };
                if let Some(item) = target {
                    let _ = camera.set_control(control.id, &item.index);
                }
            }
            _ => {}
        }
    }

    fn pick_menu_index(items: &[rscam::CtrlMenuItem], quality_mode: bool) -> Option<u32> {
        let mut best: Option<(u32, i32)> = None;
        for item in items {
            let text = item.name.to_ascii_lowercase();
            let score = if quality_mode {
                menu_quality_score(&text)
            } else {
                menu_noise_score(&text)
            };
            match best {
                None => best = Some((item.index, score)),
                Some((_, cur)) if score > cur => best = Some((item.index, score)),
                _ => {}
            }
        }
        best.map(|(idx, _)| idx)
            .or_else(|| items.last().map(|i| i.index))
    }

    fn menu_quality_score(text: &str) -> i32 {
        let mut score = 0;
        if text.contains("high") || text.contains("best") || text.contains("fine") {
            score += 40;
        }
        if text.contains("max") {
            score += 50;
        }
        if text.contains("medium") {
            score += 10;
        }
        if text.contains("low") {
            score -= 20;
        }
        score
    }

    fn menu_noise_score(text: &str) -> i32 {
        let mut score = 0;
        if text.contains("high") || text.contains("max") {
            score += 50;
        }
        if text.contains("medium") || text.contains("auto") || text.contains("on") {
            score += 25;
        }
        if text.contains("off") || text.contains("disable") || text.contains("low") {
            score -= 30;
        }
        score
    }

    fn quantize_int_to_step(value: i32, min: i32, max: i32, step: i32) -> i32 {
        let mut v = value.clamp(min, max);
        let s = step.abs().max(1);
        let offset = (v - min) / s;
        v = min + offset * s;
        v.clamp(min, max)
    }

    fn quantize_i64_to_step(value: i64, min: i64, max: i64, step: i64) -> i64 {
        let mut v = value.clamp(min, max);
        let s = step.abs().max(1);
        let offset = (v - min) / s;
        v = min + offset * s;
        v.clamp(min, max)
    }

    fn is_quality_control_name(name: &str) -> bool {
        (name.contains("jpeg") || name.contains("compression") || name.contains("quality"))
            && name.contains("quality")
    }

    fn is_noise_reduction_control_name(name: &str) -> bool {
        name.contains("noise reduction")
            || name.contains("denoise")
            || name.contains("noise_reduction")
            || name.contains("3d noise")
            || name.contains("2d noise")
    }
}
