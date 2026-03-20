//! Native entry point for FourDeers

fn main() -> Result<(), eframe::Error> {
    let native_options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_min_inner_size([800.0, 600.0])
            .with_title("FourDeers - Stereoscopic 4D Visualization"),
        ..Default::default()
    };

    eframe::run_native(
        "FourDeers",
        native_options,
        Box::new(|cc| {
            cc.egui_ctx.set_theme(eframe::egui::Theme::Dark);
            Ok(Box::new(fourdeers::FourDeersApp::new(cc)))
        }),
    )
}
