use egui_vertical_stack::VerticalStack;

struct MyApp {
    vertical_stack: VerticalStack,
}

impl MyApp {
    pub fn new() -> Self {
        Self {
            vertical_stack: VerticalStack::new()
                .min_panel_height(50.0)
                .default_panel_height(150.0),
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
       egui::SidePanel::left("left_panel").show(ctx, |ui| {
            self.vertical_stack
                .id_salt(ui.id().with("vertical_stack"))
                .body(ui, |body|{
                    body.add_panel("top", |ui|{
                        ui.label("top");
                    });
                    body.add_panel("middle", |ui|{
                        ui.style_mut().wrap_mode = Some(eframe::egui::TextWrapMode::Extend);
                        ui.label("middle with some non-wrapping text");
                    });
                    body.add_panel("bottom", |ui|{
                        ui.label("bottom");
                    });
                });
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label("main content");
        });
    }
}

fn main() -> eframe::Result {
    let native_options = eframe::NativeOptions::default();
    eframe::run_native("egui_vertical_stack - Simple", native_options, Box::new(|_cc| Ok(Box::new(MyApp::new()))))
}
