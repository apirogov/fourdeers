use eframe::egui;

pub struct ComponentColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl ComponentColor {
    #[must_use]
    pub fn to_egui_color(self) -> egui::Color32 {
        egui::Color32::from_rgba_unmultiplied(self.r, self.g, self.b, self.a)
    }
}

#[must_use]
pub fn compute_component_color(component: f32, max_abs: f32) -> ComponentColor {
    const EPSILON: f32 = 1e-6;

    if max_abs < EPSILON || component.abs() < EPSILON {
        return ComponentColor {
            r: 255,
            g: 255,
            b: 255,
            a: 220,
        };
    }

    let relative_strength = (component.abs() / max_abs).min(1.0);

    if component > 0.0 {
        let intensity = relative_strength;
        let r = crate::colors::to_u8(255.0 * (1.0 - intensity * 0.8));
        let g = crate::colors::to_u8(255.0 * (1.0 - intensity * 0.5));
        let b = 255;
        ComponentColor { r, g, b, a: 230 }
    } else {
        let intensity = relative_strength;
        let r = 255;
        let g = crate::colors::to_u8(255.0 * (1.0 - intensity * 0.6));
        let b = crate::colors::to_u8(255.0 * (1.0 - intensity * 0.6));
        ComponentColor { r, g, b, a: 230 }
    }
}
