use crate::canvas::CanvasState;
use crate::history::History;
use crate::mesh::PartVisibility;
use crate::palette::Palette;
use crate::skin::SkinModel;
use crate::tools::{Tool, ToolState};
use eframe::egui;

pub fn tool_panel(ui: &mut egui::Ui, tool_state: &mut ToolState) {
    ui.heading("Tools");
    ui.add_space(4.0);

    let tools = [
        Tool::Pencil,
        Tool::Eraser,
        Tool::Bucket,
        Tool::ColorPicker,
        Tool::Line,
        Tool::Rectangle,
        Tool::Ellipse,
        Tool::Select,
    ];

    for tool in &tools {
        let selected = tool_state.current_tool == *tool;
        let label = format!("{} ({})", tool.name(), tool.shortcut());
        if ui.selectable_label(selected, label).clicked() {
            tool_state.current_tool = *tool;
        }
    }

    ui.add_space(8.0);
    ui.separator();
    ui.add_space(4.0);

    ui.label("Brush Size:");
    let mut size = tool_state.brush_size as f32;
    ui.add(egui::Slider::new(&mut size, 1.0..=8.0).step_by(1.0));
    tool_state.brush_size = size as u8;

    ui.add_space(8.0);
    ui.separator();
    ui.add_space(4.0);

    ui.label("Mirror:");
    ui.checkbox(&mut tool_state.mirror_x, "Horizontal (X)");
    ui.checkbox(&mut tool_state.mirror_y, "Vertical (Y)");

    ui.add_space(8.0);
    ui.separator();
    ui.add_space(4.0);

    let [r, g, b, a] = tool_state.primary_color;
    let primary = egui::Color32::from_rgba_unmultiplied(r, g, b, a);
    let [r, g, b, a] = tool_state.secondary_color;
    let secondary = egui::Color32::from_rgba_unmultiplied(r, g, b, a);

    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(egui::vec2(24.0, 24.0), egui::Sense::hover());
        ui.painter().rect_filled(rect, 2.0, primary);
        ui.painter().rect_stroke(
            rect,
            2.0,
            egui::Stroke::new(1.0, egui::Color32::WHITE),
            egui::StrokeKind::Middle,
        );

        let (rect, _) = ui.allocate_exact_size(egui::vec2(24.0, 24.0), egui::Sense::hover());
        ui.painter().rect_filled(rect, 2.0, secondary);
        ui.painter().rect_stroke(
            rect,
            2.0,
            egui::Stroke::new(1.0, egui::Color32::GRAY),
            egui::StrokeKind::Middle,
        );

        if ui.button("⇄").on_hover_text("Swap colors (X)").clicked() {
            tool_state.swap_colors();
        }
    });
}

pub fn color_panel(ui: &mut egui::Ui, tool_state: &mut ToolState, palette: &mut Palette) {
    ui.heading("Color");
    ui.add_space(4.0);

    let mut color = egui::Color32::from_rgba_unmultiplied(
        tool_state.primary_color[0],
        tool_state.primary_color[1],
        tool_state.primary_color[2],
        tool_state.primary_color[3],
    );

    egui::color_picker::color_picker_color32(
        ui,
        &mut color,
        egui::color_picker::Alpha::BlendOrAdditive,
    );

    tool_state.primary_color = [color.r(), color.g(), color.b(), color.a()];

    ui.add_space(4.0);

    // Explicit Hex Input for easy copy/pasting
    let hex = format!(
        "#{:02X}{:02X}{:02X}{:02X}",
        tool_state.primary_color[0],
        tool_state.primary_color[1],
        tool_state.primary_color[2],
        tool_state.primary_color[3]
    );
    ui.horizontal(|ui| {
        ui.label("Hex:");
        let mut hex_str = hex.clone();
        let response = ui.text_edit_singleline(&mut hex_str);
        if response.lost_focus() || response.changed() {
            // Only update if it successfully parsed (e.g., when pasting a new code)
            if let Some(parsed) = parse_hex_color(&hex_str) {
                tool_state.primary_color = parsed;
            }
        }
    });

    ui.add_space(8.0);
    ui.separator();
    ui.add_space(4.0);

    ui.horizontal(|ui| {
        ui.heading("Palette");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.small_button("?")
                .on_hover_text("Left-click: select color\nRight-click: remove from palette");
        });
    });
    ui.label(
        egui::RichText::new(&palette.name)
            .small()
            .color(egui::Color32::from_rgb(150, 150, 160)),
    );
    ui.add_space(2.0);

    let cols = 6;
    let swatch_size = 18.0;
    let spacing = 2.0;
    let mut remove_index: Option<usize> = None;

    egui::Grid::new("palette_grid")
        .spacing(egui::vec2(spacing, spacing))
        .show(ui, |ui| {
            for (i, &color) in palette.colors.iter().enumerate() {
                let [r, g, b, a] = color;
                let c = egui::Color32::from_rgba_unmultiplied(r, g, b, a);
                let (rect, response) = ui.allocate_exact_size(
                    egui::vec2(swatch_size, swatch_size),
                    egui::Sense::click(),
                );
                ui.painter().rect_filled(rect, 1.0, c);
                if tool_state.primary_color == color {
                    ui.painter().rect_stroke(
                        rect,
                        1.0,
                        egui::Stroke::new(2.0, egui::Color32::WHITE),
                        egui::StrokeKind::Middle,
                    );
                }
                if response.clicked() {
                    tool_state.primary_color = color;
                }
                if response.secondary_clicked() {
                    remove_index = Some(i);
                }
                response.on_hover_text(format!("#{:02X}{:02X}{:02X} A:{}", r, g, b, a));
                if (i + 1) % cols == 0 {
                    ui.end_row();
                }
            }
        });

    // Apply deferred removal
    if let Some(idx) = remove_index {
        palette.remove_color(idx);
    }

    ui.add_space(4.0);

    ui.horizontal(|ui| {
        if ui
            .small_button("+ Add Color")
            .on_hover_text("Add current color to palette")
            .clicked()
        {
            palette.add_color(tool_state.primary_color);
        }
        if ui
            .small_button("Reset")
            .on_hover_text("Reset to default palette")
            .clicked()
        {
            palette.reset();
        }
    });

    ui.add_space(2.0);

    ui.horizontal(|ui| {
        if ui
            .small_button("Import .gpl")
            .on_hover_text("Import a GIMP palette file")
            .clicked()
        {
            if let Some(path) = crate::io::open_palette_dialog() {
                match Palette::load_from_file(&path) {
                    Ok(loaded) => {
                        palette.colors = loaded.colors;
                        palette.name = loaded.name;
                    }
                    Err(_e) => {
                        // TODO: surface error to user
                    }
                }
            }
        }
        if ui
            .small_button("Export .gpl")
            .on_hover_text("Export palette as GIMP palette file")
            .clicked()
        {
            if let Some(path) = crate::io::save_palette_dialog() {
                let _ = palette.save_to_file(&path);
            }
        }
    });

    ui.add_space(4.0);

    if !palette.recent.is_empty() {
        ui.label("Recent:");
        ui.horizontal_wrapped(|ui| {
            for &color in &palette.recent {
                let [r, g, b, a] = color;
                let c = egui::Color32::from_rgba_unmultiplied(r, g, b, a);
                let (rect, response) =
                    ui.allocate_exact_size(egui::vec2(14.0, 14.0), egui::Sense::click());
                ui.painter().rect_filled(rect, 1.0, c);
                if response.clicked() {
                    tool_state.primary_color = color;
                }
            }
        });
    }
}

pub fn layer_panel(
    ui: &mut egui::Ui,
    visibility: &mut PartVisibility,
    model: &mut SkinModel,
    canvas: &mut CanvasState,
) {
    ui.heading("Layers");
    ui.add_space(4.0);

    ui.horizontal(|ui| {
        ui.label("Model:");
        if ui
            .selectable_label(*model == SkinModel::Classic, "Classic")
            .clicked()
        {
            *model = SkinModel::Classic;
        }
        if ui
            .selectable_label(*model == SkinModel::Slim, "Slim")
            .clicked()
        {
            *model = SkinModel::Slim;
        }
    });

    ui.add_space(4.0);
    ui.checkbox(&mut canvas.show_overlay_regions, "Show overlay regions");

    ui.add_space(8.0);
    ui.label("Base Parts:");
    ui.checkbox(&mut visibility.head, "Head");
    ui.checkbox(&mut visibility.body, "Body");
    ui.checkbox(&mut visibility.right_arm, "Right Arm");
    ui.checkbox(&mut visibility.left_arm, "Left Arm");
    ui.checkbox(&mut visibility.right_leg, "Right Leg");
    ui.checkbox(&mut visibility.left_leg, "Left Leg");

    ui.add_space(8.0);
    ui.label("Overlay Parts:");
    ui.checkbox(&mut visibility.hat, "Hat");
    ui.checkbox(&mut visibility.jacket, "Jacket");
    ui.checkbox(&mut visibility.right_sleeve, "Right Sleeve");
    ui.checkbox(&mut visibility.left_sleeve, "Left Sleeve");
    ui.checkbox(&mut visibility.right_pant, "Right Pant");
    ui.checkbox(&mut visibility.left_pant, "Left Pant");

    ui.add_space(4.0);
    ui.horizontal(|ui| {
        if ui.button("All").clicked() {
            *visibility = PartVisibility::all_visible();
        }
        if ui.button("None").clicked() {
            visibility.head = false;
            visibility.body = false;
            visibility.right_arm = false;
            visibility.left_arm = false;
            visibility.right_leg = false;
            visibility.left_leg = false;
            visibility.hat = false;
            visibility.jacket = false;
            visibility.right_sleeve = false;
            visibility.left_sleeve = false;
            visibility.right_pant = false;
            visibility.left_pant = false;
        }
        if ui.button("Base Only").clicked() {
            *visibility = PartVisibility::all_visible();
            visibility.hat = false;
            visibility.jacket = false;
            visibility.right_sleeve = false;
            visibility.left_sleeve = false;
            visibility.right_pant = false;
            visibility.left_pant = false;
        }
        if ui.button("Overlay Only").clicked() {
            *visibility = PartVisibility::all_visible();
            visibility.head = false;
            visibility.body = false;
            visibility.right_arm = false;
            visibility.left_arm = false;
            visibility.right_leg = false;
            visibility.left_leg = false;
        }
    });
    ui.add_space(4.0);
}

pub fn history_panel(ui: &mut egui::Ui, history: &History) -> Option<usize> {
    ui.heading("History");
    ui.add_space(4.0);

    let undo_descs = history.undo_descriptions();
    let redo_descs = history.redo_descriptions();
    let undo_count = undo_descs.len();
    let total = undo_count + redo_descs.len();

    if total == 0 {
        ui.label(
            egui::RichText::new("No history")
                .small()
                .color(egui::Color32::from_rgb(120, 120, 130)),
        );
        return None;
    }

    let mut target: Option<usize> = None;

    egui::ScrollArea::vertical()
        .max_height(200.0)
        .stick_to_bottom(true)
        .show(ui, |ui| {
            // "Initial state" baseline entry
            let is_current = undo_count == 0 && !redo_descs.is_empty();
            let label = egui::RichText::new("▸ Initial state")
                .small()
                .color(if undo_count == 0 {
                    egui::Color32::WHITE
                } else {
                    egui::Color32::from_rgb(100, 100, 110)
                });
            if ui.selectable_label(is_current, label).clicked() && undo_count > 0 {
                target = Some(0);
            }

            // Undo entries (already done — shown normally)
            for (i, desc) in undo_descs.iter().enumerate() {
                let entry_idx = i + 1; // 1-based position
                let is_latest = i == undo_count - 1;
                let text = if is_latest {
                    format!("▸ {desc}")
                } else {
                    format!("  {desc}")
                };
                let label = egui::RichText::new(text)
                    .small()
                    .color(egui::Color32::from_rgb(220, 220, 230));
                if ui.selectable_label(is_latest, label).clicked() && !is_latest {
                    target = Some(entry_idx);
                }
            }

            // Redo entries (future — shown dimmed)
            for (i, desc) in redo_descs.iter().enumerate() {
                let entry_idx = undo_count + i + 1;
                let label = egui::RichText::new(format!("  {desc}"))
                    .small()
                    .color(egui::Color32::from_rgb(80, 80, 90));
                if ui.selectable_label(false, label).clicked() {
                    target = Some(entry_idx);
                }
            }
        });

    target
}

fn parse_hex_color(s: &str) -> Option<[u8; 4]> {
    let s = s.trim().trim_start_matches('#');
    match s.len() {
        6 => {
            let r = u8::from_str_radix(&s[0..2], 16).ok()?;
            let g = u8::from_str_radix(&s[2..4], 16).ok()?;
            let b = u8::from_str_radix(&s[4..6], 16).ok()?;
            Some([r, g, b, 255])
        }
        8 => {
            let r = u8::from_str_radix(&s[0..2], 16).ok()?;
            let g = u8::from_str_radix(&s[2..4], 16).ok()?;
            let b = u8::from_str_radix(&s[4..6], 16).ok()?;
            let a = u8::from_str_radix(&s[6..8], 16).ok()?;
            Some([r, g, b, a])
        }
        _ => None,
    }
}
