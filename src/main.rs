mod app;
mod camera;
mod canvas;
mod history;
mod io;
mod mesh;
mod palette;
mod panels;
mod reference;
mod renderer;
mod selection;
mod skin;
mod tools;
mod uv_map;

fn main() -> eframe::Result {
    env_logger::init();

    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1400.0, 900.0])
            .with_min_inner_size([800.0, 600.0])
            .with_title("Skinner"),
        multisampling: 4,
        depth_buffer: 24,
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };

    eframe::run_native(
        "Skinner",
        options,
        Box::new(|cc| Ok(Box::new(app::SkinnerApp::new(cc)))),
    )
}
