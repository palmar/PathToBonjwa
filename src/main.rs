mod analytics;
mod library;
mod parser;
mod settings;
mod ui;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([800.0, 620.0])
            .with_min_inner_size([520.0, 400.0])
            .with_drag_and_drop(true),
        ..Default::default()
    };

    eframe::run_native(
        "PathToBonjwa — BW Replay Analyzer",
        options,
        Box::new(|cc| {
            cc.egui_ctx.set_visuals(ui::bw_visuals());
            Ok(Box::new(ui::App::default()))
        }),
    )
}
