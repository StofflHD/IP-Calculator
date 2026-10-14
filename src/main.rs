#![windows_subsystem = "windows"]

mod app;
mod ipv4;
mod ipv6;

fn main() -> eframe::Result {
    let mut viewport = eframe::egui::ViewportBuilder::default()
        .with_title("IP Calculator")
        .with_app_id("ip-calculator")
        .with_inner_size([540.0, 700.0])
        .with_min_inner_size([520.0, 620.0]);
    if let Ok(icon) =
        eframe::icon_data::from_png_bytes(include_bytes!("../packaging/ip-calculator.png"))
    {
        viewport = viewport.with_icon(icon);
    }
    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };
    eframe::run_native(
        "IP Calculator",
        options,
        Box::new(|cc| Ok(Box::new(app::App::new(cc)))),
    )
}
