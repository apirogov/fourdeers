#[must_use]
pub fn format_component_value(value: f32) -> String {
    if value.abs() < 1e-6 {
        "0.00".to_string()
    } else if value.abs() >= 100.0 {
        format!("{value:.0}")
    } else if value.abs() >= 10.0 {
        format!("{value:.1}")
    } else {
        format!("{value:.2}")
    }
}

#[must_use]
pub fn format_magnitude(magnitude: f32) -> String {
    format_component_value(magnitude)
}
