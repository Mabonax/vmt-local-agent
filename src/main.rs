#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod tool_ui;
use vmt_local_agent::{
    agent, ollama, portable_skills, projects, skills, store, tool_packs, types, workspace,
};

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if let Some(index) = args.iter().position(|a| a == "--data-dir") {
        store::configure_data_dir(
            args.get(index + 1)
                .ok_or_else(|| anyhow::anyhow!("--data-dir requires a path"))?
                .into(),
        )?;
    }
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
            .with_inner_size([1360.0, 760.0])
            .with_position([24.0, 24.0])
            .with_min_inner_size([860.0, 620.0]),
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
