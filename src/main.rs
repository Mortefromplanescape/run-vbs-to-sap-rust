mod app;

use app::ui::MyApp;
use eframe::egui;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        resizable: true,
        initial_window_size: Some(egui::vec2(1000.0, 500.0)),
        vsync: false,
        ..Default::default()
    };

    eframe::run_native(
        app::constants::LABEL_MAIN_WINDOW,
        options,
        Box::new(|cc| {
            cc.egui_ctx.set_visuals(egui::Visuals::dark());
            Box::new(MyApp::new(cc))
        }),
    )
}