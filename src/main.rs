#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
use vmt_local_agent::{agent, ollama, store, types, workspace};

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if let Some(index) = args.iter().position(|a| a == "--smoke-test") {
        return agent::smoke_test(
            args.get(index + 1)
                .map(String::as_str)
                .unwrap_or("deepseek-r1-local:7b"),
        );
    }
    let workspace = args
        .iter()
        .position(|a| a == "--workspace")
        .and_then(|i| args.get(i + 1))
        .cloned();
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title(concat!("VMT Local Agent ", env!("CARGO_PKG_VERSION")))
            .with_inner_size([1240.0, 740.0])
            .with_min_inner_size([940.0, 650.0]),
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };
    eframe::run_native(
        "VMT Local Agent",
        options,
        Box::new(move |cc| Ok(Box::new(app::Desktop::new(cc, workspace)))),
    )
    .map_err(|e| anyhow::anyhow!("Could not open the desktop window: {e}"))
}
