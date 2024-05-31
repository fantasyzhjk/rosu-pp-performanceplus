pub fn is_roughly_equal(a: f64, b: f64) -> bool {
    a * 1.25 > b && a / 1.25 < b
}

pub fn is_ratio_equal(ratio: f64, a: f64, b: f64) -> bool {
    a + 5.0 > ratio * b && a - 5.0 < ratio * b
}

pub fn is_ratio_equal_greater(ratio: f64, a: f64, b: f64) -> bool {
    a + 5.0 > ratio * b
}

pub fn is_ratio_equal_less(ratio: f64, a: f64, b: f64) -> bool {
    a - 5.0 < ratio * b
}

pub fn is_null_or_nan(nullable_double: Option<f64>) -> bool {
    match nullable_double {
        Some(value) => value.is_nan(),
        None => true,
    }
}

/// A boolean function that produces non-binary results when the value being checked is between the 100% True and 100% False thresholds.
/// 
/// # Arguments
/// 
/// * `value` - The value being evaluated.
/// * `transition_start` - If the value is at or below this, the result is False.
/// * `transition_interval` - Length of the interval through which the result gradually transitions from False to True.
/// 
/// # Returns
/// 
/// Returns a double value from [0, 1] where 0 is 100% False, and 1 is 100% True.
pub fn transition_to_true(value: f64, transition_start: f64, transition_interval: f64) -> f64 {
    if value <= transition_start {
        0.0
    } else if value >= transition_start + transition_interval {
        1.0
    } else {
        (-((value - transition_start) * std::f64::consts::PI / transition_interval).cos() + 1.0) / 2.0
    }
}

/// A boolean function that produces non-binary results when the value being checked is between the 100% True and 100% False thresholds.
/// 
/// # Arguments
/// 
/// * `value` - The value being evaluated.
/// * `transition_start` - If the value is at or below this, the result is True.
/// * `transition_interval` - Length of the interval through which the result gradually transitions from True to False.
/// 
/// # Returns
/// 
/// Returns a double value from [0, 1] where 0 is 100% False, and 1 is 100% True.
pub fn transition_to_false(value: f64, transition_start: f64, transition_interval: f64) -> f64 {
    if value <= transition_start {
        1.0
    } else if value >= transition_start + transition_interval {
        0.0
    } else {
        (((value - transition_start) * std::f64::consts::PI / transition_interval).cos() + 1.0) / 2.0
    }
}