use std::cell::Cell;

use vertigo::{Computed, Value, get_driver};

/// Seconds going on with the frames the browser draws, from 0: for animations, f. ex. a turn
/// by `time * speed`. It ticks only while something on the page depends on it, so a scene
/// standing still asks for no frames, and each call makes a clock of its own, starting when it's
/// first shown.
///
/// ```rust
/// use vertigo_3d::{Transform, frame_time, math::Quat};
///
/// // A turn a second, while shown.
/// let turning = frame_time()
///     .map(|time| Transform::from_rotation(Quat::from_rotation_y(time * std::f32::consts::TAU)));
/// ```
pub fn frame_time() -> Computed<f32> {
    Value::with_connect(0.0, |seconds| {
        let seconds = seconds.clone();
        let start = Cell::new(None);
        get_driver().on_animation_frame(move |time| {
            let first = start.get().unwrap_or(time);
            start.set(Some(first));
            seconds.set(((time - first) / 1000.0) as f32);
        })
    })
}
