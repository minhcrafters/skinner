use eframe::egui;
use eframe::glow;
use std::sync::Arc;

use crate::camera::OrbitCamera;
use crate::canvas::{self, CanvasState};
use crate::history::{History, HistoryAction, HistoryEntry};
use crate::mesh::PartVisibility;
use crate::palette::Palette;
use crate::reference::ReferenceImage;
use crate::renderer::Renderer3D;
use crate::selection::Selection;
use crate::skin::SkinTexture;
use crate::tools::{Tool, ToolState};
use crate::{io, panels};

pub struct SkinnerApp {
    skin: SkinTexture,
    tool_state: ToolState,
    history: History,
    palette: Palette,
    canvas_state: CanvasState,
    camera: OrbitCamera,
    renderer: Arc<egui::mutex::Mutex<Renderer3D>>,
    part_visibility: PartVisibility,
    selection: Selection,
    current_file: Option<std::path::PathBuf>,
    status_message: String,
    show_3d: bool,
    error_msg: Option<String>,
    reference_images: Vec<ReferenceImage>,
}

impl SkinnerApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut visuals = egui::Visuals::dark();
        visuals.window_shadow = egui::epaint::Shadow::NONE;
        visuals.panel_fill = egui::Color32::from_rgb(30, 30, 35);
        visuals.faint_bg_color = egui::Color32::from_rgb(35, 35, 42);
        visuals.extreme_bg_color = egui::Color32::from_rgb(20, 20, 24);
        visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(40, 40, 48);
        visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(50, 50, 60);
        visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(65, 65, 80);
        visuals.widgets.active.bg_fill = egui::Color32::from_rgb(80, 80, 100);
        visuals.selection.bg_fill = egui::Color32::from_rgb(60, 90, 140);
        cc.egui_ctx.set_visuals(visuals);

        let mut style = (*cc.egui_ctx.style()).clone();
        style.spacing.item_spacing = egui::vec2(6.0, 4.0);
        style.spacing.window_margin = egui::Margin::same(8);
        cc.egui_ctx.set_style(style);

        let gl = cc.gl.as_ref().expect("Need glow backend");
        let renderer = Arc::new(egui::mutex::Mutex::new(Renderer3D::new(gl)));

        Self {
            skin: SkinTexture::new(),
            tool_state: ToolState::new(),
            history: History::new(),
            palette: Palette::new(),
            canvas_state: CanvasState::new(),
            camera: OrbitCamera::new(),
            renderer,
            part_visibility: PartVisibility::all_visible(),
            selection: Selection::new(),
            current_file: None,
            status_message: "Ready".to_string(),
            show_3d: true,
            error_msg: None,
            reference_images: Vec::new(),
        }
    }

    fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        ctx.input(|i| {
            if i.key_pressed(egui::Key::B) && !i.modifiers.ctrl {
                self.tool_state.current_tool = Tool::Pencil;
            }
            if i.key_pressed(egui::Key::E) && !i.modifiers.ctrl {
                self.tool_state.current_tool = Tool::Eraser;
            }
            if i.key_pressed(egui::Key::G) && !i.modifiers.ctrl {
                self.tool_state.current_tool = Tool::Bucket;
            }
            if i.key_pressed(egui::Key::I) && !i.modifiers.ctrl {
                self.tool_state.current_tool = Tool::ColorPicker;
            }
            if i.key_pressed(egui::Key::L) && !i.modifiers.ctrl {
                self.tool_state.current_tool = Tool::Line;
            }
            if i.key_pressed(egui::Key::U) && !i.modifiers.ctrl {
                self.tool_state.current_tool = Tool::Rectangle;
            }
            if i.key_pressed(egui::Key::O) && !i.modifiers.ctrl {
                self.tool_state.current_tool = Tool::Ellipse;
            }
            if i.key_pressed(egui::Key::M) && !i.modifiers.ctrl {
                self.tool_state.current_tool = Tool::Select;
            }
            if i.key_pressed(egui::Key::X) && !i.modifiers.ctrl {
                self.tool_state.swap_colors();
            }

            if i.key_pressed(egui::Key::OpenBracket) {
                self.tool_state.brush_size = (self.tool_state.brush_size.saturating_sub(1)).max(1);
            }
            if i.key_pressed(egui::Key::CloseBracket) {
                self.tool_state.brush_size = (self.tool_state.brush_size + 1).min(8);
            }

            // Escape — cancel selection
            if i.key_pressed(egui::Key::Escape) && self.selection.active {
                let sel_before = self.selection.snapshot();
                let changes = self.selection.cancel(&mut self.skin);
                if !changes.is_empty() {
                    self.history.push(HistoryEntry::from_changes_with_selection(
                        "Cancel selection".to_string(),
                        changes,
                        sel_before,
                        None,
                    ));
                }
                self.status_message = "Selection cancelled".to_string();
            }
        });

        // Undo/Redo and selection shortcuts (with Ctrl modifier)
        ctx.input(|i| {
            if i.modifiers.ctrl && i.key_pressed(egui::Key::Z) {
                if i.modifiers.shift {
                    if self.history.redo(&mut self.skin, &mut self.selection) {
                        self.status_message = "Redo".to_string();
                    }
                } else {
                    if self.history.undo(&mut self.skin, &mut self.selection) {
                        self.status_message = "Undo".to_string();
                    }
                }
            }

            // Save
            if i.modifiers.ctrl && i.key_pressed(egui::Key::S) {
                self.save_current();
            }

            // Open
            if i.modifiers.ctrl && i.key_pressed(egui::Key::O) {
                self.open_file();
            }

            // New
            if i.modifiers.ctrl && i.key_pressed(egui::Key::N) {
                self.new_skin();
            }

            // Ctrl+D — deselect
            if i.modifiers.ctrl && i.key_pressed(egui::Key::D) {
                let sel_before = self.selection.snapshot();
                let changes = self.selection.deselect(&mut self.skin);
                if !changes.is_empty() {
                    self.history.push(HistoryEntry::from_changes_with_selection(
                        "Deselect".to_string(),
                        changes,
                        sel_before,
                        None,
                    ));
                }
                self.status_message = "Deselected".to_string();
            }

            // Ctrl+C — copy selection
            if i.modifiers.ctrl && i.key_pressed(egui::Key::C) {
                if self.selection.active && self.selection.pixels.is_some() {
                    self.selection.copy_to_clipboard();
                    self.status_message = "Copied to clipboard".to_string();
                }
            }

            // Ctrl+V — paste from clipboard
            if i.modifiers.ctrl && i.key_pressed(egui::Key::V) {
                if self.selection.has_clipboard() {
                    let sel_before = self.selection.snapshot();
                    let changes = self.selection.paste_from_clipboard(&mut self.skin);
                    let sel_after = self.selection.snapshot();
                    if !changes.is_empty() {
                        self.history.push(HistoryEntry::from_changes_with_selection(
                            "Paste".to_string(),
                            changes,
                            sel_before,
                            sel_after,
                        ));
                    }
                    self.tool_state.current_tool = Tool::Select;
                    self.status_message = "Pasted from clipboard".to_string();
                }
            }

            // Ctrl+X — cut (copy then we already have it cut)
            if i.modifiers.ctrl && i.key_pressed(egui::Key::X) {
                if self.selection.active && self.selection.pixels.is_some() {
                    self.selection.copy_to_clipboard();
                    self.status_message = "Cut to clipboard".to_string();
                }
            }
        });
    }

    fn new_skin(&mut self) {
        // Commit any active selection
        if self.selection.active {
            let sel_before = self.selection.snapshot();
            let changes = self.selection.deselect(&mut self.skin);
            if !changes.is_empty() {
                self.history.push(HistoryEntry::from_changes_with_selection(
                    "Deselect".to_string(),
                    changes,
                    sel_before,
                    None,
                ));
            }
        }
        self.skin = SkinTexture::new();
        self.history = History::new();
        self.current_file = None;
        self.status_message = "New skin created".to_string();
    }

    fn open_file(&mut self) {
        if let Some(path) = io::open_file_dialog() {
            match io::load_skin(&path) {
                Ok(skin) => {
                    self.skin = skin;
                    self.history = History::new();
                    self.selection = Selection::new();
                    self.current_file = Some(path.clone());
                    self.status_message = format!("Opened: {}", path.display());
                }
                Err(e) => {
                    self.error_msg = Some(format!("Failed to open: {e}"));
                }
            }
        }
    }

    fn save_current(&mut self) {
        if let Some(ref path) = self.current_file.clone() {
            match io::save_skin(path, &self.skin) {
                Ok(()) => {
                    self.status_message = format!("Saved: {}", path.display());
                }
                Err(e) => {
                    self.error_msg = Some(format!("Failed to save: {e}"));
                }
            }
        } else {
            self.save_as();
        }
    }

    fn save_as(&mut self) {
        if let Some(path) = io::save_file_dialog() {
            match io::save_skin(&path, &self.skin) {
                Ok(()) => {
                    self.current_file = Some(path.clone());
                    self.status_message = format!("Saved: {}", path.display());
                }
                Err(e) => {
                    self.error_msg = Some(format!("Failed to save: {e}"));
                }
            }
        }
    }

    fn menu_bar(&mut self, ui: &mut egui::Ui) {
        egui::menu::bar(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui.button("New (Ctrl+N)").clicked() {
                    self.new_skin();
                    ui.close_menu();
                }
                if ui.button("Open... (Ctrl+O)").clicked() {
                    self.open_file();
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("Save (Ctrl+S)").clicked() {
                    self.save_current();
                    ui.close_menu();
                }
                if ui.button("Save As...").clicked() {
                    self.save_as();
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("Open Reference Image...").clicked() {
                    if let Some(path) = io::open_image_dialog() {
                        match ReferenceImage::load(&path) {
                            Ok(ref_img) => {
                                self.reference_images.push(ref_img);
                            }
                            Err(e) => {
                                self.error_msg =
                                    Some(format!("Failed to load reference image: {e}"));
                            }
                        }
                    }
                    ui.close_menu();
                }
            });

            ui.menu_button("Edit", |ui| {
                let can_undo = self.history.can_undo();
                let can_redo = self.history.can_redo();
                if ui
                    .add_enabled(can_undo, egui::Button::new("Undo (Ctrl+Z)"))
                    .clicked()
                {
                    self.history.undo(&mut self.skin, &mut self.selection);
                    ui.close_menu();
                }
                if ui
                    .add_enabled(can_redo, egui::Button::new("Redo (Ctrl+Shift+Z)"))
                    .clicked()
                {
                    self.history.redo(&mut self.skin, &mut self.selection);
                    ui.close_menu();
                }

                ui.separator();

                let has_sel = self.selection.active && self.selection.pixels.is_some();
                if ui
                    .add_enabled(has_sel, egui::Button::new("Deselect (Ctrl+D)"))
                    .clicked()
                {
                    let sel_before = self.selection.snapshot();
                    let changes = self.selection.deselect(&mut self.skin);
                    if !changes.is_empty() {
                        self.history.push(HistoryEntry::from_changes_with_selection(
                            "Deselect".to_string(),
                            changes,
                            sel_before,
                            None,
                        ));
                    }
                    ui.close_menu();
                }
                if ui
                    .add_enabled(has_sel, egui::Button::new("Copy (Ctrl+C)"))
                    .clicked()
                {
                    self.selection.copy_to_clipboard();
                    ui.close_menu();
                }
                if ui
                    .add_enabled(has_sel, egui::Button::new("Cut (Ctrl+X)"))
                    .clicked()
                {
                    self.selection.copy_to_clipboard();
                    ui.close_menu();
                }
                let has_clip = self.selection.has_clipboard();
                if ui
                    .add_enabled(has_clip, egui::Button::new("Paste (Ctrl+V)"))
                    .clicked()
                {
                    let sel_before = self.selection.snapshot();
                    let changes = self.selection.paste_from_clipboard(&mut self.skin);
                    let sel_after = self.selection.snapshot();
                    if !changes.is_empty() {
                        self.history.push(HistoryEntry::from_changes_with_selection(
                            "Paste".to_string(),
                            changes,
                            sel_before,
                            sel_after,
                        ));
                    }
                    self.tool_state.current_tool = Tool::Select;
                    ui.close_menu();
                }

                ui.separator();
                ui.label("Transform Selection:");

                // Helper closure: apply a transform to the selection with undo support
                macro_rules! do_transform {
                    ($desc:expr, $op:expr) => {
                        if let Some(before) = self.selection.snapshot() {
                            $op;
                            if let Some(after) = self.selection.snapshot() {
                                self.history.push(HistoryEntry {
                                    description: $desc.to_string(),
                                    action: HistoryAction::SelectionTransform { before, after },
                                });
                            }
                        }
                    };
                }

                if ui
                    .add_enabled(has_sel, egui::Button::new("Flip Horizontal"))
                    .clicked()
                {
                    do_transform!("Flip Horizontal", self.selection.flip_h());
                    ui.close_menu();
                }
                if ui
                    .add_enabled(has_sel, egui::Button::new("Flip Vertical"))
                    .clicked()
                {
                    do_transform!("Flip Vertical", self.selection.flip_v());
                    ui.close_menu();
                }
                if ui
                    .add_enabled(has_sel, egui::Button::new("Rotate 90° CW"))
                    .clicked()
                {
                    do_transform!("Rotate CW", self.selection.rotate_cw());
                    ui.close_menu();
                }
                if ui
                    .add_enabled(has_sel, egui::Button::new("Rotate 90° CCW"))
                    .clicked()
                {
                    do_transform!("Rotate CCW", self.selection.rotate_ccw());
                    ui.close_menu();
                }

                ui.separator();
                ui.label("Skew:");
                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(has_sel, egui::Button::new("← H"))
                        .on_hover_text("Skew horizontally left (-1px)")
                        .clicked()
                    {
                        do_transform!("Skew H left", self.selection.skew_h(-1));
                    }
                    if ui
                        .add_enabled(has_sel, egui::Button::new("H →"))
                        .on_hover_text("Skew horizontally right (+1px)")
                        .clicked()
                    {
                        do_transform!("Skew H right", self.selection.skew_h(1));
                    }
                });
                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(has_sel, egui::Button::new("↑ V"))
                        .on_hover_text("Skew vertically up (-1px)")
                        .clicked()
                    {
                        do_transform!("Skew V up", self.selection.skew_v(-1));
                    }
                    if ui
                        .add_enabled(has_sel, egui::Button::new("V ↓"))
                        .on_hover_text("Skew vertically down (+1px)")
                        .clicked()
                    {
                        do_transform!("Skew V down", self.selection.skew_v(1));
                    }
                });

                ui.separator();
                ui.label("Stretch:");
                if has_sel {
                    let mut sw = self.selection.w;
                    let mut sh = self.selection.h;
                    let orig_w = sw;
                    let orig_h = sh;
                    ui.horizontal(|ui| {
                        ui.label("W:");
                        ui.add(egui::DragValue::new(&mut sw).range(1..=64).speed(0.2));
                        ui.label("H:");
                        ui.add(egui::DragValue::new(&mut sh).range(1..=64).speed(0.2));
                    });
                    if sw != orig_w || sh != orig_h {
                        do_transform!("Stretch", self.selection.stretch(sw, sh));
                    }
                    ui.horizontal(|ui| {
                        if ui.small_button("2×").on_hover_text("Double size").clicked() {
                            let nw = (self.selection.w * 2).min(64);
                            let nh = (self.selection.h * 2).min(64);
                            do_transform!("Stretch 2×", self.selection.stretch(nw, nh));
                        }
                        if ui.small_button("½×").on_hover_text("Halve size").clicked() {
                            let nw = (self.selection.w / 2).max(1);
                            let nh = (self.selection.h / 2).max(1);
                            do_transform!("Stretch ½×", self.selection.stretch(nw, nh));
                        }
                    });
                } else {
                    ui.add_enabled(false, egui::Button::new("(no selection)"));
                }
            });

            ui.menu_button("View", |ui| {
                ui.checkbox(&mut self.canvas_state.show_grid, "Grid");
                ui.checkbox(&mut self.canvas_state.show_region_labels, "Region Labels");
                ui.checkbox(&mut self.show_3d, "3D Preview");
                ui.separator();
                if ui.button("Reset Zoom & Pan").clicked() {
                    self.canvas_state.zoom = 8.0;
                    self.canvas_state.pan_offset = egui::Vec2::ZERO;
                    ui.close_menu();
                }
                if ui.button("Reset Camera").clicked() {
                    self.camera = OrbitCamera::new();
                    ui.close_menu();
                }
            });
        });
    }

    fn show_3d_viewport(&mut self, ui: &mut egui::Ui) {
        let available = ui.available_size();
        let viewport_size = egui::vec2(available.x.max(200.0), available.y.max(200.0));

        let bg_color = egui::Color32::from_rgb(45, 45, 55);
        let (rect, response) = ui.allocate_exact_size(viewport_size, egui::Sense::click_and_drag());
        ui.painter().rect_filled(rect, 0.0, bg_color);

        if response.dragged_by(egui::PointerButton::Primary) {
            let delta = response.drag_delta();
            self.camera.orbit(-delta.x, delta.y);
        }
        if response.dragged_by(egui::PointerButton::Middle) {
            let delta = response.drag_delta();
            self.camera.pan(delta.x, delta.y);
        }
        let scroll = ui.input(|i| i.smooth_scroll_delta.y);
        if response.hovered() && scroll.abs() > 0.0 {
            self.camera.zoom(scroll * 0.005);
        }

        let aspect = rect.width() / rect.height();
        let mvp = self.camera.mvp(aspect);
        let mvp_array: [f32; 16] = mvp.to_cols_array();

        // Sync texture if dirty (or selection changed)
        // Composite the floating selection onto the skin data for 3D display
        // so selected pixels don't disappear from the 3D preview.
        {
            let mut pixels = self.skin.pixels_as_bytes();
            if self.selection.active {
                if let Some(ref sel_pixels) = self.selection.pixels {
                    for dy in 0..self.selection.h {
                        for dx in 0..self.selection.w {
                            let sx = self.selection.x + dx as i32;
                            let sy = self.selection.y + dy as i32;
                            if sx >= 0 && sx < 64 && sy >= 0 && sy < 64 {
                                let idx = (dy * self.selection.w + dx) as usize;
                                let [r, g, b, a] = sel_pixels[idx];
                                if a > 0 {
                                    let offset = ((sy as u32 * 64 + sx as u32) * 4) as usize;
                                    pixels[offset] = r;
                                    pixels[offset + 1] = g;
                                    pixels[offset + 2] = b;
                                    pixels[offset + 3] = a;
                                }
                            }
                        }
                    }
                }
            }
            self.renderer.lock().set_pending_pixels(pixels);
        }

        // Sync model type if sidebar changed it
        {
            let mut renderer = self.renderer.lock();
            renderer.set_model_type(self.skin.model);
            renderer.set_visibility(self.part_visibility.clone());
        }

        let renderer = Arc::clone(&self.renderer);
        let screen_size = ui.ctx().screen_rect().size();
        let pixels_per_point = ui.ctx().pixels_per_point();

        let clip_rect = [
            rect.left() * pixels_per_point,
            rect.top() * pixels_per_point,
            rect.right() * pixels_per_point,
            rect.bottom() * pixels_per_point,
        ];
        let screen_px = [
            (screen_size.x * pixels_per_point) as u32,
            (screen_size.y * pixels_per_point) as u32,
        ];

        let vp_x = (rect.left() * pixels_per_point) as i32;
        let vp_y = ((screen_size.y - rect.bottom()) * pixels_per_point) as i32;
        let vp_w = (rect.width() * pixels_per_point) as i32;
        let vp_h = (rect.height() * pixels_per_point) as i32;

        let callback = egui::PaintCallback {
            rect,
            callback: Arc::new(eframe::egui_glow::CallbackFn::new(move |_info, painter| {
                use glow::HasContext;
                let gl = painter.gl();
                unsafe {
                    gl.viewport(vp_x, vp_y, vp_w, vp_h);
                }
                renderer.lock().paint(gl, &mvp_array, screen_px, clip_rect);
            })),
        };
        ui.painter().add(callback);

        ui.painter().text(
            rect.left_top() + egui::vec2(8.0, 8.0),
            egui::Align2::LEFT_TOP,
            "3D Preview (Drag to rotate, Scroll to zoom, Middle to pan)",
            egui::FontId::proportional(11.0),
            egui::Color32::from_rgba_unmultiplied(200, 200, 200, 180),
        );
    }
}

impl eframe::App for SkinnerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.handle_shortcuts(ctx);

        // Error dialog
        if let Some(ref msg) = self.error_msg.clone() {
            egui::Window::new("Error")
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.label(msg);
                    if ui.button("OK").clicked() {
                        self.error_msg = None;
                    }
                });
        }

        for ref_img in &mut self.reference_images {
            if let Some(color) = ref_img.show_window(ctx) {
                self.tool_state.primary_color = color;
            }
        }
        self.reference_images.retain(|r| r.open);

        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            self.menu_bar(ui);
        });

        egui::TopBottomPanel::bottom("status_bar")
            .exact_height(24.0)
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.label(&self.status_message);
                    ui.separator();
                    ui.label(format!("Tool: {}", self.tool_state.current_tool.name()));
                    ui.separator();
                    if let Some((px, py)) = self.canvas_state.hovered_pixel {
                        let color = self.skin.get_pixel(px, py);
                        ui.label(format!("({px}, {py})"));
                        ui.separator();
                        ui.label(format!(
                            "RGBA({}, {}, {}, {})",
                            color[0], color[1], color[2], color[3]
                        ));
                        if let Some(ref region) = self.canvas_state.hovered_region {
                            ui.separator();
                            ui.label(region.as_str());
                        }
                    }
                    if self.selection.active {
                        ui.separator();
                        ui.label(format!(
                            "Sel: {}×{} at ({},{})",
                            self.selection.w, self.selection.h, self.selection.x, self.selection.y
                        ));
                    }
                    ui.separator();
                    ui.label(format!(
                        "History: {}/{}",
                        self.history.undo_count(),
                        self.history.undo_count() + self.history.redo_count()
                    ));
                });
            });

        egui::SidePanel::left("tool_panel")
            .default_width(140.0)
            .min_width(120.0)
            .resizable(true)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    panels::tool_panel(ui, &mut self.tool_state);
                });
                ui.add_space(8.0);
                ui.separator();
                ui.add_space(4.0);
                let history_target = panels::history_panel(ui, &self.history);
                if let Some(target) = history_target {
                    let current = self.history.undo_count();
                    if target < current {
                        self.history
                            .undo_to(target, &mut self.skin, &mut self.selection);
                        self.status_message = "Undo".to_string();
                    } else if target > current {
                        self.history
                            .redo_to(target, &mut self.skin, &mut self.selection);
                        self.status_message = "Redo".to_string();
                    }
                }
            });

        egui::SidePanel::right("properties_panel")
            .default_width(180.0)
            .min_width(150.0)
            .resizable(true)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    panels::color_panel(ui, &mut self.tool_state, &mut self.palette);
                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(4.0);
                    panels::layer_panel(
                        ui,
                        &mut self.part_visibility,
                        &mut self.skin.model,
                        &mut self.canvas_state,
                    );
                });
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            if self.show_3d {
                let total_height = ui.available_height();
                let split = total_height * 0.55;

                ui.allocate_ui(egui::vec2(ui.available_width(), split), |ui| {
                    egui::Frame::NONE
                        .fill(egui::Color32::from_rgb(25, 25, 30))
                        .show(ui, |ui| {
                            canvas::show_canvas(
                                ui,
                                &mut self.skin,
                                &mut self.canvas_state,
                                &mut self.tool_state,
                                &mut self.history,
                                &mut self.selection,
                                ctx,
                            );
                        });
                });

                ui.separator();

                self.show_3d_viewport(ui);
            } else {
                egui::Frame::NONE
                    .fill(egui::Color32::from_rgb(25, 25, 30))
                    .show(ui, |ui| {
                        canvas::show_canvas(
                            ui,
                            &mut self.skin,
                            &mut self.canvas_state,
                            &mut self.tool_state,
                            &mut self.history,
                            &mut self.selection,
                            ctx,
                        );
                    });
            }
        });

        // Mark skin clean after all views have read it
        if self.skin.is_dirty() {
            self.skin.mark_clean();
        }

        // Request repaint for smooth 3D viewport
        if self.show_3d {
            ctx.request_repaint();
        }
    }

    fn on_exit(&mut self, gl: Option<&glow::Context>) {
        if let Some(gl) = gl {
            self.renderer.lock().destroy(gl);
        }
    }
}
