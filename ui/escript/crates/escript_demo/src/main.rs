mod demo_text_edit;
mod demo_text_edit_manual;
mod machine;

use eframe::egui;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "escript Demo App",
        native_options,
        Box::new(|cc| Ok(Box::new(EscriptDemoApp::new(cc)))),
    )?;

    Ok(())
}

#[derive(Default)]
struct EscriptDemoApp {
    text_edit_demo: demo_text_edit::TextEditDemo,
    text_edit_demo_manual: demo_text_edit_manual::TextEditDemoManual,
}

impl EscriptDemoApp {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self::default()
    }

    fn show_demo_in_window(ctx: &egui::Context, demo: &mut impl Demo) {
        egui::Window::new(demo.name()).show(ctx, |ui| {
            let mut took_s: f64 = 0.0;
            egui::Frame::new()
                .fill(ui.visuals().faint_bg_color)
                .inner_margin(4.0)
                .show(ui, |ui| {
                    let start = std::time::Instant::now();
                    demo.ui(ui);
                    let end = std::time::Instant::now();
                    took_s = (end - start).as_secs_f64();
                });

            ui.separator();
            ui.label(format!("in {:.3} ms", took_s * 1000.0));
        });
    }
}

impl eframe::App for EscriptDemoApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Hello World!");
            Self::show_demo_in_window(ctx, &mut self.text_edit_demo);
            Self::show_demo_in_window(ctx, &mut self.text_edit_demo_manual);
        });
    }
}

pub(crate) trait Demo {
    /// `&'static` so we can also use it as a key to store open/close state.
    fn name(&self) -> &'static str;
    fn ui(&mut self, ui: &mut egui::Ui);
}
