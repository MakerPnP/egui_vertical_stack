use std::boxed::Box;
use std::collections::HashMap;
use std::hash::Hash;

use egui::scroll_area::ScrollBarVisibility;
use egui::{Color32, CornerRadius, Id, Rect, ScrollArea, Sense, Stroke, StrokeKind, Ui, UiBuilder, Vec2};

/// A component that displays multiple panels stacked vertically, each with resize handles, all contained within
/// a scroll area.
///
/// This is very useful for layouts that need multiple 'panel' views.
///
/// The amount of panels is dynamic and can be added or removed at runtime.
///
/// Example use:
/// ```no_run
/// use egui_vertical_stack::VerticalStack;
///
/// struct MyApp {
///     vertical_stack: VerticalStack,
/// }
///
/// impl MyApp {
///     pub fn new() -> Self {
///         Self {
///             vertical_stack: VerticalStack::new()
///                 .min_panel_height(50.0)
///                 .default_panel_height(150.0),
///         }
///     }
/// }
///
/// impl eframe::App for MyApp {
///     fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
///         egui::SidePanel::left("left_panel").show(ctx, |ui| {
///             self.vertical_stack
///                 .id_salt(ui.id().with("vertical_stack"))
///                 .body(ui, |body|{
///
///                     // Panels can be conditionally added/removed at runtime.
///                     // Panels are added to the stack in the same order as the calls to `add_panel`
///                     // Each panel needs a unique ID, which is generated from the ID salt and the panel's hash
///
///                     body.add_panel("top", |ui|{
///                         ui.label("top");
///                     });
///                     body.add_panel("middle", |ui|{
///                         ui.style_mut().wrap_mode = Some(eframe::egui::TextWrapMode::Extend);
///                         ui.label("middle with some non-wrapping text");
///                     });
///                     body.add_panel("bottom", |ui|{
///                         ui.label("bottom");
///                     });
///                 });
///         });
///         egui::CentralPanel::default().show(ctx, |ui| {
///             ui.label("main content");
///         });
///     }
/// }
///
/// fn main() -> eframe::Result {
///     let native_options = eframe::NativeOptions::default();
///     eframe::run_native("egui_vertical_stack - Simple", native_options, Box::new(|_cc| Ok(Box::new(MyApp::new()))))
/// }
/// ```
///
/// For a more complete example, check out the `demos` folder in the source.
#[derive(Debug)]
pub struct VerticalStack {
    //
    // settings
    //
    id_source: Id,
    /// Maximum height for the scroll area.
    max_height: Option<f32>,
    min_panel_height: f32,
    max_panel_height: Option<f32>,
    default_panel_height: f32,
    scroll_bar_visibility: ScrollBarVisibility,

    //
    // from sizing pass
    //
    panel_heights: HashMap<Id, f32>,
    /// (width, height)
    content_sizes: HashMap<Id, (f32, f32)>,

    //
    // dragging state
    //
    active_drag_handle: Option<usize>,
    drag_in_progress: bool,
    drag_start_y: Option<f32>,
    drag_start_height: Option<f32>,

    //
    // other state
    //
    initialized: bool,
    last_available_height: f32,
    last_panel_count: usize,
}

impl VerticalStack {
    pub fn new() -> Self {
        Self {
            id_source: Id::new("vertical_stack"),
            max_height: None,
            min_panel_height: 50.0,
            max_panel_height: None,
            default_panel_height: 100.0,
            scroll_bar_visibility: ScrollBarVisibility::VisibleWhenNeeded,

            panel_heights: HashMap::new(),
            content_sizes: HashMap::new(),

            active_drag_handle: None,
            drag_in_progress: false,
            drag_start_y: None,
            drag_start_height: None,

            initialized: false,
            last_available_height: 0.0,
            last_panel_count: 0,
        }
    }

    /// Sets a custom ID salt for the stack.
    pub fn id_salt(&mut self, id: impl Hash) -> &mut Self {
        self.id_source = Id::new(id);
        self
    }

    /// Sets the maximum height for the scroll area.
    /// If None, will use all available height.
    pub fn max_height(mut self, height: Option<f32>) -> Self {
        self.max_height = height;
        self
    }

    /// Sets the minimum height for panels.
    pub fn min_panel_height(mut self, height: f32) -> Self {
        self.min_panel_height = height;
        self
    }

    /// Sets the maximum height for individual panels.
    /// If None, panels can grow as large as needed (or up to the entire scroll area).
    pub fn max_panel_height(mut self, height: Option<f32>) -> Self {
        self.max_panel_height = height;
        self
    }

    /// Set the default height for new panels.
    pub fn default_panel_height(mut self, height: f32) -> Self {
        self.default_panel_height = height;
        self
    }

    /// Adjust the scroll-area's scrollbar visibility, the default is 'when needed', but using 'always visible' will
    /// likely yield a better UX.
    pub fn scroll_bar_visibility(mut self, visibility: ScrollBarVisibility) -> Self {
        self.scroll_bar_visibility = visibility;
        self
    }

    /// The main function to render the stack and add panels.
    pub fn body<F>(&mut self, ui: &mut Ui, mut collect_panels: F)
    where
        F: FnMut(&mut StackBodyBuilder),
    {
        let available_rect = ui.available_rect_before_wrap();
        let available_height = match self.max_height {
            Some(max_height) => max_height.min(available_rect.height()),
            None => available_rect.height(),
        };

        let mut body = StackBodyBuilder {
            panels: Vec::new(),
        };

        // Collect panel functions
        collect_panels(&mut body);

        // Get panel count
        let panel_count = body.panels.len();

        // Early return if no panels
        if panel_count == 0 {
            return;
        }

        let seen_all_panels = body
            .panels
            .iter()
            .all(|(id, _fn)| self.panel_heights.contains_key(id));

        // Check if we need a sizing pass
        if !self.initialized
            || (self.last_available_height - available_height).abs() > 1.0
            || panel_count != self.last_panel_count
            || !seen_all_panels
        {
            self.last_available_height = available_height;
            self.last_panel_count = panel_count;

            // Run a sizing pass to measure content heights
            self.do_sizing_pass(ui, &mut body);

            ui.ctx().request_discard("sizing");
            // Mark as initialized
            self.initialized = true;

            // avoid final rendering on the sizing pass
            return;
        }

        // Now do the actual rendering with known content heights
        self.do_render_pass(ui, body, available_height);
    }

    /// Perform a sizing pass to measure content heights without rendering
    fn do_sizing_pass(&mut self, ui: &mut Ui, body: &mut StackBodyBuilder) {
        //self.content_sizes.clear();
        // Create a temporary UI for measuring content heights
        ui.allocate_ui(Vec2::new(ui.available_width(), 0.0), |ui| {
            // Get the available width for panels
            let panel_width = ui.available_width();

            // Measure each panel's content
            for (hash, panel_fn) in body.panels.iter_mut() {
                // Always respect the minimum height from the beginning
                let mut panel_height = *self
                    .panel_heights
                    .get(hash)
                    .unwrap_or(&self.default_panel_height);

                // Also ensure ALL existing panel heights respect minimum height
                panel_height = panel_height.max(self.min_panel_height);

                // Also apply max_panel_height if configured
                if let Some(max_panel_height) = self.max_panel_height {
                    panel_height = panel_height.min(max_panel_height);
                }

                // Create a temporary rect with max height for measurement
                let panel_rect = Rect::from_min_size(ui.cursor().min, Vec2::new(panel_width, f32::MAX));

                // Create a new child UI with sizing pass enabled
                let mut measuring_ui = ui.new_child(
                    UiBuilder::new()
                        .max_rect(panel_rect)
                        .sizing_pass(),
                );

                // Call the panel function to measure its size
                panel_fn(&mut measuring_ui);

                // Get the measured content height
                let content_rect = measuring_ui.min_rect();
                let content_height = content_rect.height();
                self.content_sizes
                    .insert(*hash, (content_rect.width(), content_height));

                // Update the panel height
                self.panel_heights
                    .insert(*hash, panel_height);
            }
        });
    }

    /// Render the actual UI with known content heights
    fn do_render_pass(&mut self, ui: &mut Ui, body: StackBodyBuilder, available_height: f32) {
        let inner_margin = 4.0;

        // Handle drag state
        let pointer_is_down = ui.input(|i| i.pointer.any_down());

        if self.drag_in_progress && !pointer_is_down {
            self.drag_in_progress = false;
            self.active_drag_handle = None;
        }

        let (max_content_width, _max_content_height) = self
            .content_sizes
            .values()
            .fold((0.0_f32, 0.0_f32), |acc, (w, h)| (acc.0.max(*w), acc.1.max(*h)));

        // Create a ScrollArea with the available height
        ScrollArea::both()
            .id_salt(self.id_source.with("scroll_area"))
            .max_height(available_height)
            .scroll_bar_visibility(self.scroll_bar_visibility)
            .auto_shrink([false, true])
            .show(ui, |ui| {
                let scroll_area_rect_before_wrap = ui.available_rect_before_wrap();
                #[cfg(feature = "layout_debugging")]
                {
                    let debug_stroke = Stroke::new(1.0, Color32::PURPLE);
                    ui.painter().rect(
                        scroll_area_rect_before_wrap,
                        CornerRadius::ZERO,
                        Color32::TRANSPARENT,
                        debug_stroke,
                        StrokeKind::Outside,
                    );
                }

                // Use vertical layout with no spacing
                ui.spacing_mut().item_spacing.y = 0.0;

                // Get the available rect for the panel content
                let panel_rect = ui.available_rect_before_wrap();

                for (idx, (id, mut panel_fn)) in body.panels.into_iter().enumerate() {
                    // Get panel height (already has min_height applied above)
                    let panel_height = *self.panel_heights.get(&id).unwrap();

                    let desired_size = Vec2::new(panel_rect.width(), panel_height);
                    ui.allocate_ui(desired_size, |ui| {
                        // Set a min height for the panel content
                        ui.set_min_height(panel_height);

                        let frame_stroke_width = 1.0;
                        let intial_frame_width = max_content_width + ((inner_margin + frame_stroke_width) * 2.0);
                        let frame_width = intial_frame_width.max(scroll_area_rect_before_wrap.width());

                        // without this, the right hand side of the frame will not be visible when the scroll area is narrower than the content
                        ui.set_min_width(frame_width);

                        // Draw the panel frame
                        let mut frame_rect = ui.max_rect();
                        frame_rect.max.x = frame_rect.min.x + frame_width;
                        frame_rect.max.y = frame_rect.min.y + panel_height;

                        let stroke = Stroke::new(
                            frame_stroke_width,
                            ui.visuals()
                                .widgets
                                .noninteractive
                                .bg_stroke
                                .color,
                        );
                        ui.painter().rect(
                            frame_rect,
                            CornerRadius::ZERO,
                            Color32::TRANSPARENT,
                            stroke,
                            StrokeKind::Outside,
                        );

                        let content_rect = frame_rect.shrink(inner_margin);
                        let mut panel_ui = ui.new_child(UiBuilder::new().max_rect(content_rect));

                        // Intersect the clip rectangles to prevent the last panel content overflowing the bottom of
                        // the scroll area when a maximum height is set on the scroll area.
                        let panel_rect = content_rect.intersect(ui.clip_rect());
                        #[cfg(feature = "layout_debugging")]
                        {
                            let debug_stroke = Stroke::new(1.0, Color32::GREEN);
                            ui.painter().rect(
                                panel_rect,
                                CornerRadius::ZERO,
                                Color32::TRANSPARENT,
                                debug_stroke,
                                StrokeKind::Outside,
                            );
                        }
                        panel_ui.set_clip_rect(panel_rect);
                        panel_fn(&mut panel_ui);
                    });

                    self.add_resize_handle(ui, idx, &id);
                }
            });
    }

    fn add_resize_handle(&mut self, ui: &mut Ui, panel_idx: usize, id: &Id) {
        let handle_height = 7.0;

        // For the last panel handle, we don't need to check the next panel's index
        let is_last_panel = panel_idx == self.panel_heights.len() - 1;

        // Skip if not valid (except for last panel)
        if !is_last_panel && (panel_idx >= self.panel_heights.len() || panel_idx + 1 >= self.panel_heights.len()) {
            return;
        }

        // Calculate handle rect
        let available_rect = ui.available_rect_before_wrap();
        let handle_rect = Rect::from_min_size(available_rect.min, Vec2::new(available_rect.width(), handle_height));

        // Allocate exact size with drag sense
        let (rect, response) = ui.allocate_exact_size(handle_rect.size(), Sense::drag());

        // Set the resize cursor when hovering or dragging
        if response.hovered() || response.dragged() {
            ui.ctx()
                .set_cursor_icon(egui::CursorIcon::ResizeVertical);
        }

        // Draw a small horizontal rule to indicate the drag handle
        // Calculate exact position for the 1px line with 3px padding above and below
        let painter = ui.painter();
        let handle_stroke = if response.hovered() || response.dragged() {
            Stroke::new(1.0, Color32::WHITE)
        } else {
            Stroke::new(
                1.0,
                ui.visuals()
                    .widgets
                    .noninteractive
                    .bg_stroke
                    .color,
            )
        };

        // Position line exactly in the middle of the handle area
        let line_y = rect.min.y + (rect.height() / 2.0);

        // Draw a thin line that's always visible
        painter.line_segment(
            [
                egui::Pos2::new(rect.left(), line_y),
                egui::Pos2::new(rect.right(), line_y),
            ],
            handle_stroke,
        );

        #[cfg(feature = "layout_debugging")]
        {
            // Debug visualization of handle rect to see its exact bounds (can be removed in production)
            let debug_stroke = Stroke::new(1.0, Color32::YELLOW);
            painter.rect_stroke(rect, 0.0, debug_stroke, StrokeKind::Outside);
        }

        // Check if this is the active drag handle
        let is_active_handle = self.active_drag_handle == Some(panel_idx);

        // Handle drag start
        if response.drag_started() {
            self.drag_in_progress = true;
            self.active_drag_handle = Some(panel_idx);

            // Store initial values
            if let Some(pointer_pos) = ui.ctx().pointer_latest_pos() {
                self.drag_start_y = Some(pointer_pos.y);

                let panel_height = *self.panel_heights.get(id).unwrap();
                self.drag_start_height = Some(panel_height);
            }
        }

        // Handle ongoing drag
        if is_active_handle && self.drag_in_progress && response.dragged() {
            if let (Some(start_y), Some(start_height)) = (self.drag_start_y, self.drag_start_height) {
                if let Some(current_pos) = ui.ctx().pointer_latest_pos() {
                    let panel_height = self.panel_heights.get_mut(id).unwrap();

                    // Calculate delta from initial position
                    let total_delta = current_pos.y - start_y;

                    // Apply delta to the initial height
                    let mut new_height = (start_height + total_delta).max(self.min_panel_height);

                    // Apply maximum constraint if set
                    if let Some(max_panel_height) = self.max_panel_height {
                        new_height = new_height.min(max_panel_height);
                    }

                    // Update the panel height
                    *panel_height = new_height;
                }
            }
        }

        // Handle drag end
        if is_active_handle && response.drag_stopped() {
            self.drag_in_progress = false;
            self.active_drag_handle = None;
            self.drag_start_y = None;
            self.drag_start_height = None;
        }
    }
}

/// Collects panel closures and ids for later use.
pub struct StackBodyBuilder {
    panels: Vec<(Id, Box<dyn FnMut(&mut Ui)>)>,
}

impl StackBodyBuilder {
    /// Add a panel to the stack with the given content.
    ///
    /// Each panel should have its own id, usually just the panel index, but if you re-arrange
    /// panels then use something that uniquely identifies the panel, e.g a name.
    ///
    /// Panels can be conditionally added/removed at runtime by calling/not-calling this method.
    /// Panels are added to the stack in the same order as the calls to `add_panel`
    pub fn add_panel<F>(&mut self, id_salt: impl Hash, add_contents: F)
    where
        F: FnMut(&mut Ui) + 'static,
    {
        let id = Id::new(id_salt);
        // Box the function and store it for later execution
        self.panels
            .push((id, Box::new(add_contents)));
    }
}

impl Default for VerticalStack {
    fn default() -> Self {
        Self::new()
    }
}
