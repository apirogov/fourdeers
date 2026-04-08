//! WASM entry point

use wasm_bindgen::prelude::*;

const CANVAS_ELEMENT_ID: &str = "the_canvas_id";

#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();

    let canvas = web_sys::window()
        .expect("running in browser with window object")
        .document()
        .expect("window has document")
        .get_element_by_id(CANVAS_ELEMENT_ID)
        .expect("canvas element exists in DOM")
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .expect("element is an HtmlCanvasElement");

    let web_options = eframe::WebOptions::default();

    wasm_bindgen_futures::spawn_local(async move {
        run_app(canvas, web_options).await;
    });
}

async fn run_app(canvas: web_sys::HtmlCanvasElement, web_options: eframe::WebOptions) {
    eframe::WebRunner::new()
        .start(
            canvas,
            web_options,
            Box::new(|cc| {
                cc.egui_ctx.set_theme(eframe::egui::Theme::Dark);
                Ok(Box::new(crate::app::FourDeersApp::new(cc)))
            }),
        )
        .await
        .expect("Failed to start eframe");
}
