//! A small demonstration of the frame, with no window of its own.
//!
//! ```text
//! cargo run -p eh3d-dock --example dock_demo -- --sector pcb --workspace Edit --lang es --shot board.svg
//! cargo run -p eh3d-dock --example dock_demo --features gui -- --sector cnc --frames 3
//! ```
//!
//! `--shot FILE` writes a picture (SVG) of the factory layout of a sector; `--text` prints the layout as the settings file keeps it; with the
//! feature `gui` the example also draws a few frames of the real egui code with no window and says how many panels it drew. To see the frame in
//! a window, give `Dock::show` the `Ui` of an `eframe` application (see `docs/DOCKING.md`).

use eh3d_dock::core::DockCore;
use eh3d_dock::preview::layout_svg;
use eh3d_dock::sectors::Sector;
use eh3d_view::texts::Lang;

fn arg(args: &[String], name: &str) -> Option<String> {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1))
        .cloned()
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!("dock_demo [--sector printer3d|pcb|cad|cnc|pnp|laser] [--workspace NAME] [--lang CODE] [--shot FILE.svg] [--text] [--frames N]");
        return;
    }
    let sector_name = arg(&args, "--sector").unwrap_or_else(|| "pcb".to_string());
    let Some(sector) = Sector::ALL.into_iter().find(|s| s.name() == sector_name) else {
        eprintln!("unknown sector: {sector_name}");
        std::process::exit(2);
    };
    let lang = arg(&args, "--lang")
        .and_then(|c| Lang::from_code(&c))
        .unwrap_or(Lang::En);
    let mut core = DockCore::new(sector.panels(), sector.workspaces());
    core.lang = lang;
    if let Some(w) = arg(&args, "--workspace") {
        if core.workspaces().names().contains(&w.as_str()) {
            let _ = core.apply(&eh3d_dock::menu::DockAction::SwitchWorkspace(w));
        } else {
            eprintln!(
                "workspaces of {}: {}",
                sector.name(),
                core.workspaces().names().join(", ")
            );
            std::process::exit(2);
        }
    }
    println!(
        "sector {} - workspace {} - {} panels",
        sector.name(),
        core.workspaces().current_name(),
        core.defs().len()
    );
    if args.iter().any(|a| a == "--text") {
        print!("{}", core.layout_text());
    }
    if let Some(file) = arg(&args, "--shot") {
        let svg = layout_svg(core.layout(), core.defs(), lang, 1280.0, 720.0);
        if let Err(e) = std::fs::write(&file, svg) {
            eprintln!("cannot write {file}: {e}");
            std::process::exit(1);
        }
        println!("picture written to {file}");
    }
    #[cfg(feature = "gui")]
    {
        let frames: usize = arg(&args, "--frames")
            .and_then(|n| n.parse().ok())
            .unwrap_or(3);
        let mut dock = eh3d_dock::gui::Dock::new(sector.panels(), sector.workspaces());
        dock.set_language(lang);
        let drawn = std::rc::Rc::new(std::cell::Cell::new(0u32));
        for d in sector.panels() {
            let count = drawn.clone();
            let name = d.id.clone();
            dock.register(&d.id, move |ui| {
                count.set(count.get() + 1);
                ui.label(name.as_str());
            });
        }
        let ctx = egui::Context::default();
        for i in 0..frames {
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1280.0, 720.0),
                )),
                time: Some(i as f64 * 0.05),
                ..Default::default()
            };
            let out = ctx.run_ui(input, |ui| dock.show(ui));
            out.drop_without_applying_deltas();
        }
        println!(
            "{frames} frames drawn with egui, {} panel bodies called",
            drawn.get()
        );
    }
}
