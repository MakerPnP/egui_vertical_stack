use std::sync::{Arc, Mutex};
use eframe::CreationContext;
use eframe::egui::{Color32, CornerRadius, Stroke, StrokeKind};
use egui::{Frame, Style};
use egui_vertical_stack::VerticalStack;
struct MyApp {
    vertical_stack: VerticalStack,
    enabled_panels: [bool; 4],
    panel_order: [usize; 4],
    vertical_stack_settings: VerticalStackSettings,

    example_state: Arc<Mutex<bool>>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct VerticalStackSettings {
    min_panel_height: f32,
    default_panel_height: f32,
    max_panel_height: Option<f32>,
    max_height: Option<f32>,
}

impl Default for VerticalStackSettings {
    fn default() -> Self {
        Self {
            min_panel_height: 50.0,
            default_panel_height: 150.0,
            max_panel_height: Some(300.0),
            max_height: Some(300.0),
        }
    }
}

impl MyApp {
    pub fn new(cc: &CreationContext) -> Self {
        cc.egui_ctx.style_mut(|style| {
            // Set solid scrollbars for the entire app
            style.spacing.scroll = eframe::egui::style::ScrollStyle::solid();

        });
        
        let vertical_stack_settings = VerticalStackSettings::default();
        
        Self {
            vertical_stack: Self::make_vertical_stack(&vertical_stack_settings),
            enabled_panels: [true, true, false, false],
            vertical_stack_settings,
            example_state: Arc::new(Mutex::new(false)),
            panel_order: [0, 1, 2, 3],
        }
    }
    
    pub fn make_vertical_stack(settings: &VerticalStackSettings) -> VerticalStack {
        VerticalStack::new()
            .min_panel_height(settings.min_panel_height)
            .default_panel_height(settings.default_panel_height)
            .max_panel_height(settings.max_panel_height)
            .max_height(settings.max_height)
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        eframe::egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            eframe::egui::menu::bar(ui, |ui| {
                eframe::egui::Sides::new().show(
                    ui,
                    |ui| {
                        ui.label("Vertical panel layout demo.");
                    },
                    |ui| {
                        egui::widgets::global_theme_preference_switch(ui);
                    },
                );

            });
        });
        
        egui::SidePanel::left("left_panel")
            .default_width(150.0)
            .show(ctx, |ui| {
                    self.vertical_stack
                        .id_salt(ui.id().with("vertical_stack"))
                        .body(ui, {
                            |body|{
                                for (panel_index, enabled) in self.panel_order.iter().zip(self.enabled_panels) {
                                    if !enabled {
                                        continue
                                    }
                                    
                                    // panels are always displayed in the order they are added to the stack
                                    
                                    match panel_index {
                                        0 => body.add_panel(panel_index, {
                                            // clone the state, for the closure to have access to it
                                            let example_state = self.example_state.clone();
                                            move |ui| {
                                                let mut state = example_state.lock().unwrap();
                                                ui.heading("Panel 1");
                                                ui.separator();
                                                ui.checkbox(&mut state, "example state");
                                            }
                                        }),
                                        1 => body.add_panel(panel_index, |ui|{
                                            ui.style_mut().wrap_mode = Some(eframe::egui::TextWrapMode::Extend);
                                            ui.heading("Panel 2");
                                            ui.separator();
                                            ui.label("panel with some non-wrapping text");
                                        }),
                                        2 => body.add_panel(panel_index, |ui|{
                                            let rect = ui.max_rect();

                                            let debug_stroke = Stroke::new(1.0, Color32::LIGHT_GRAY);
                                            ui.painter().rect(
                                                rect,
                                                CornerRadius::ZERO,
                                                Color32::DARK_GRAY,
                                                debug_stroke,
                                                StrokeKind::Inside
                                            );
                                            ui.heading("Panel 3");
                                            ui.separator();
                                            ui.label("Panel with painting");
                                        }),
                                        3 => body.add_panel(panel_index, |ui| {
                                            ui.heading("Panel 4");
                                            ui.separator();
                                            ui.label("panel\nwith\nlots\nof\nlines\nto\nsee");
                                        }),
                                        _ => unreachable!(),
                                    }
                                }
                            }
                        });
                    ui.label("This label is below the stack");
            });
        
        egui::SidePanel::right("right_panel").show(ctx, |ui| {
            ui.heading("Right Panel");

            // Scrollable content
            egui::ScrollArea::both()
                .max_height(100.0)
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    for i in 0..20 {
                        ui.label(format!("Scrollable Item {}", i));
                    }
                });

            // Label BELOW the scroll area
            ui.label("This label is below the scroll area.");
        });
        
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label("Use the drag handles below each panel to re-size the panel above the handle.");
            ui.add_space(10.0);
            Frame::group(&Style::default())
                .show(ui, |ui| {
                    ui.label("You can enable/disable panels at runtime, or re-order them using these controls");
                    let panel_count = self.enabled_panels.len();
                    
                    let mut panel_order = self.panel_order.clone();
                    for (index, (panel_index, enabled)) in self.panel_order.iter_mut().zip(self.enabled_panels.iter_mut()).enumerate() {
                        let is_first = index == 0;
                        let is_last = index == panel_count - 1;
                        
                        ui.horizontal(|ui| {
                            ui.checkbox(enabled, format!("Enable panel {}", *panel_index + 1));
                            ui.add_enabled_ui(!is_first, |ui| if ui.button("Up").clicked() {
                                panel_order.swap(index - 1, index);
                            });
                            ui.add_enabled_ui(!is_last, |ui| if ui.button("Down").clicked() {
                                panel_order.swap(index, index + 1);
                            });
                        });
                    }
                    
                    if panel_order != self.panel_order {
                        println!("new panel order: {:?}", panel_order);
                        self.panel_order = panel_order;
                    }
                });
            
            Frame::group(&Style::default())
                .show(ui, |ui| {
                    ui.label("You can change the settings for the vertical stack at runtime using these controls");
                    ui.label("Note: this will reset the layout of the panels.");
                    
                    let initial_settings = self.vertical_stack_settings;
                    
                    ui.add(egui::Slider::new(&mut self.vertical_stack_settings.min_panel_height, 0.0..=1000.0).text("Min panel height"));
                    
                    let mut max_panel_height_enabled = self.vertical_stack_settings.max_panel_height.is_some();
                    if ui.checkbox(&mut max_panel_height_enabled, "Enable max panel height").changed() {
                        match max_panel_height_enabled {
                            true => self.vertical_stack_settings.max_panel_height = Some(100.0),
                            false => self.vertical_stack_settings.max_panel_height = None,
                        }
                    }
                    ui.add_enabled_ui(max_panel_height_enabled, |ui| {
                        if let Some(max_panel_height) = &mut self.vertical_stack_settings.max_panel_height {
                            ui.add(egui::Slider::new(max_panel_height, 0.0..=1000.0).text("Max panel height"));
                        }
                    });

                    let mut max_height_enabled = self.vertical_stack_settings.max_height.is_some();
                    if ui.checkbox(&mut max_height_enabled, "Enable max height").changed() {
                        match max_height_enabled {
                            true => self.vertical_stack_settings.max_height = Some(200.0),
                            false => self.vertical_stack_settings.max_height = None,
                        }
                    }

                    ui.add_enabled_ui(max_height_enabled, |ui| {
                        if let Some(max_height) = &mut self.vertical_stack_settings.max_height {
                            ui.add(egui::Slider::new(max_height, 0.0..=1000.0).text("Max height"));
                        }
                    });
                    ui.add(egui::Slider::new(&mut self.vertical_stack_settings.default_panel_height, 0.0..=1000.0).text("Default panel height"));
                    
                    if !initial_settings.eq(&self.vertical_stack_settings) {
                        self.vertical_stack = Self::make_vertical_stack(&self.vertical_stack_settings);
                    }
                });
        });
    }
}

fn main() -> eframe::Result {
    let native_options = eframe::NativeOptions::default();
    eframe::run_native("egui_vertical_stack - Demo", native_options, Box::new(|cc| Ok(Box::new(MyApp::new(cc)))))
}