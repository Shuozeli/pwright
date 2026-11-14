use std::num::NonZeroU32;

pub(super) const CLICK_STEPS_ERROR: &str = "steps must be an integer in [1, 10000]";
const MAX_CLICK_STEPS: u32 = 10_000;

pub(super) fn bounded_click_steps(value: f64) -> Result<NonZeroU32, &'static str> {
    if !value.is_finite()
        || value.fract() != 0.0
        || value < 1.0
        || value > f64::from(MAX_CLICK_STEPS)
    {
        return Err(CLICK_STEPS_ERROR);
    }
    NonZeroU32::new(value as u32).ok_or(CLICK_STEPS_ERROR)
}
