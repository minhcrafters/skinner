use crate::history::{History, HistoryEntry};
use crate::selection::Selection;
use crate::skin::SkinTexture;
use crate::tools::{self, Tool, ToolState};
use crate::uv_map;
/// 2D canvas panel: renders the 64×64 skin texture with zoom, grid, region labels,
/// and handles mouse interaction for painting tools.
use eframe::egui;

pub struct CanvasState {
    pub zoom: f32,
    pub show_grid: bool,
    pub show_region_labels: bool,
    pub show_overlay_regions: bool,
    pub hovered_pixel: Option<(u32, u32)>,
    pub hovered_region: Option<String>,
    pub texture_handle: Option<egui::TextureHandle>,
    /// Pan offset in screen pixels (allows dragging the canvas freely)
    pub pan_offset: egui::Vec2,
}

impl CanvasState {
    pub fn new() -> Self {
        Self {
            zoom: 8.0,
            show_grid: true,
            show_region_labels: true,
            show_overlay_regions: true,
            hovered_pixel: None,
            hovered_region: None,
            texture_handle: None,
            pan_offset: egui::Vec2::ZERO,
        }
    }
}

/// Render the 2D canvas and handle tool interaction.
/// Returns true if the skin was modified.
pub fn show_canvas(
    ui: &mut egui::Ui,
    skin: &mut SkinTexture,
    canvas: &mut CanvasState,
    tool_state: &mut ToolState,
    history: &mut History,
    selection: &mut Selection,
    ctx: &egui::Context,
) -> bool {
    let mut modified = false;

    // Ensure texture handle exists
    if canvas.texture_handle.is_none() || skin.is_dirty() {
        let img = skin.to_color_image();
        if let Some(ref mut handle) = canvas.texture_handle {
            handle.set(img, egui::TextureOptions::NEAREST);
        } else {
            canvas.texture_handle =
                Some(ctx.load_texture("skin_2d", img, egui::TextureOptions::NEAREST));
        }
    }

    // Canvas toolbar
    ui.horizontal(|ui| {
        ui.label("Zoom:");
        if ui.button("−").clicked() {
            canvas.zoom = (canvas.zoom / 2.0).max(1.0);
        }
        ui.label(format!("{:.0}×", canvas.zoom));
        if ui.button("+").clicked() {
            canvas.zoom = (canvas.zoom * 2.0).min(32.0);
        }
        ui.separator();
        ui.checkbox(&mut canvas.show_grid, "Grid");
        ui.checkbox(&mut canvas.show_region_labels, "Labels");
    });

    ui.separator();

    // The canvas occupies a virtual area that the user can pan around.
    // We allocate the full available size for the interaction region,
    // then offset the drawn content by `pan_offset`.
    let available = ui.available_size();
    let canvas_pixel_size = egui::vec2(64.0 * canvas.zoom, 64.0 * canvas.zoom);

    // Allocate the full available area for interaction (pan, paint, hover)
    let (response, painter) = ui.allocate_painter(available, egui::Sense::click_and_drag());
    let interaction_rect = response.rect;

    // Compute where the skin texture sits after panning
    let canvas_origin = interaction_rect.left_top() + canvas.pan_offset;
    let canvas_rect = egui::Rect::from_min_size(canvas_origin, canvas_pixel_size);

    // Handle panning with middle mouse button or right mouse button
    if response.dragged_by(egui::PointerButton::Middle)
        || response.dragged_by(egui::PointerButton::Secondary)
    {
        canvas.pan_offset += response.drag_delta();
    }

    // Handle zoom with scroll wheel (zoom towards cursor)
    let scroll = ui.input(|i| i.smooth_scroll_delta.y);
    if response.hovered() && scroll.abs() > 0.0 {
        let old_zoom = canvas.zoom;
        let factor = (1.0 + scroll * 0.002).clamp(0.9, 1.1);
        canvas.zoom = (canvas.zoom * factor).clamp(1.0, 32.0);
        // Zoom towards the cursor position
        if let Some(pointer) = response.hover_pos() {
            let cursor_in_canvas = pointer - canvas_origin;
            let scale = canvas.zoom / old_zoom;
            let new_cursor_in_canvas = cursor_in_canvas * scale;
            canvas.pan_offset -= new_cursor_in_canvas - cursor_in_canvas;
        }
    }

    // Draw checkerboard background for transparency
    draw_checkerboard(&painter, canvas_rect, canvas.zoom);

    // Draw the skin texture
    if let Some(ref tex) = canvas.texture_handle {
        painter.image(
            tex.id(),
            canvas_rect,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            egui::Color32::WHITE,
        );
    }

    // Draw floating selection preview (the lifted pixels rendered on top)
    if selection.active {
        if let Some(ref pixels) = selection.pixels {
            for dy in 0..selection.h {
                for dx in 0..selection.w {
                    let sx = selection.x + dx as i32;
                    let sy = selection.y + dy as i32;
                    if sx >= 0 && sx < 64 && sy >= 0 && sy < 64 {
                        let idx = (dy * selection.w + dx) as usize;
                        let [r, g, b, a] = pixels[idx];
                        if a > 0 {
                            let cell = egui::Rect::from_min_size(
                                canvas_rect.left_top()
                                    + egui::vec2(sx as f32 * canvas.zoom, sy as f32 * canvas.zoom),
                                egui::vec2(canvas.zoom, canvas.zoom),
                            );
                            painter.rect_filled(
                                cell,
                                0.0,
                                egui::Color32::from_rgba_unmultiplied(r, g, b, a),
                            );
                        }
                    }
                }
            }
        }
    }

    // Draw grid
    if canvas.show_grid && canvas.zoom >= 4.0 {
        draw_grid(&painter, canvas_rect, canvas.zoom);
    }

    // Draw region outlines and labels
    if canvas.show_region_labels {
        draw_region_labels(
            &painter,
            canvas_rect,
            canvas.zoom,
            canvas.show_overlay_regions,
            skin.model == crate::skin::SkinModel::Slim,
        );
    }

    // Draw selection marching ants
    if selection.active {
        draw_selection_outline(&painter, canvas_rect, canvas.zoom, selection, ctx);
    }
    // Draw selection-defining preview rectangle
    if selection.defining {
        if let (Some((sx, sy)), Some(pos)) = (selection.define_start, response.hover_pos()) {
            let local = pos - canvas_rect.left_top();
            let ex = (local.x / canvas.zoom).floor().clamp(0.0, 63.0) as u32;
            let ey = (local.y / canvas.zoom).floor().clamp(0.0, 63.0) as u32;
            let min_x = sx.min(ex);
            let min_y = sy.min(ey);
            let max_x = sx.max(ex);
            let max_y = sy.max(ey);
            let sel_rect = egui::Rect::from_min_max(
                canvas_rect.left_top()
                    + egui::vec2(min_x as f32 * canvas.zoom, min_y as f32 * canvas.zoom),
                canvas_rect.left_top()
                    + egui::vec2(
                        (max_x + 1) as f32 * canvas.zoom,
                        (max_y + 1) as f32 * canvas.zoom,
                    ),
            );
            painter.rect_stroke(
                sel_rect,
                0.0,
                egui::Stroke::new(
                    1.5,
                    egui::Color32::from_rgba_unmultiplied(100, 180, 255, 200),
                ),
                egui::StrokeKind::Middle,
            );
        }
    }

    // Handle hover
    canvas.hovered_pixel = None;
    canvas.hovered_region = None;
    if let Some(pos) = response.hover_pos() {
        let local = pos - canvas_rect.left_top();
        let px = (local.x / canvas.zoom).floor() as i32;
        let py = (local.y / canvas.zoom).floor() as i32;
        if px >= 0 && px < 64 && py >= 0 && py < 64 {
            let px = px as u32;
            let py = py as u32;
            canvas.hovered_pixel = Some((px, py));
            canvas.hovered_region = uv_map::region_at_pixel(px, py, skin.model == crate::skin::SkinModel::Slim);

            // Draw hover highlight
            let cell_rect = egui::Rect::from_min_size(
                canvas_rect.left_top()
                    + egui::vec2(px as f32 * canvas.zoom, py as f32 * canvas.zoom),
                egui::vec2(canvas.zoom, canvas.zoom),
            );
            painter.rect_stroke(
                cell_rect,
                0.0,
                egui::Stroke::new(2.0, egui::Color32::from_rgb(255, 255, 0)),
                egui::StrokeKind::Middle,
            );
        }
    }

    // ──── Selection tool interaction ────
    if tool_state.current_tool == Tool::Select {
        if let Some((px, py)) = canvas.hovered_pixel {
            let px_i = px as i32;
            let py_i = py as i32;

            if response.drag_started_by(egui::PointerButton::Primary) {
                if selection.active && selection.pixels.is_some() && selection.contains(px_i, py_i)
                {
                    // Start dragging the floating selection
                    selection.dragging = true;
                    selection.drag_offset = (px_i - selection.x, py_i - selection.y);
                } else {
                    // Commit any existing selection before starting a new one
                    if selection.active && selection.pixels.is_some() {
                        let sel_before = selection.snapshot();
                        let changes = selection.commit(skin);
                        if !changes.is_empty() {
                            history.push(HistoryEntry::from_changes_with_selection(
                                "Commit selection".to_string(),
                                changes,
                                sel_before,
                                None,
                            ));
                            modified = true;
                        }
                    }
                    // Start defining a new selection
                    selection.defining = true;
                    selection.define_start = Some((px, py));
                }
            }

            if response.dragged_by(egui::PointerButton::Primary) {
                if selection.dragging {
                    // Move the floating selection
                    let new_x = px_i - selection.drag_offset.0;
                    let new_y = py_i - selection.drag_offset.1;
                    selection.x = new_x;
                    selection.y = new_y;
                }
                // defining case is handled by the preview drawing above
            }
        }

        // On drag release
        if response.drag_stopped() {
            if selection.defining {
                // Finalize the selection rectangle and cut
                if let Some((sx, sy)) = selection.define_start {
                    if let Some(pos) = response.interact_pointer_pos() {
                        let local = pos - canvas_rect.left_top();
                        let ex = (local.x / canvas.zoom).floor().clamp(0.0, 63.0) as u32;
                        let ey = (local.y / canvas.zoom).floor().clamp(0.0, 63.0) as u32;
                        if sx != ex || sy != ey {
                            let changes = selection.select_and_cut(sx, sy, ex, ey, skin);
                            let sel_after = selection.snapshot();
                            if !changes.is_empty() {
                                history.push(HistoryEntry::from_changes_with_selection(
                                    "Select region".to_string(),
                                    changes,
                                    None, // was inactive before
                                    sel_after,
                                ));
                                modified = true;
                            }
                        } else {
                            selection.defining = false;
                            selection.define_start = None;
                        }
                    } else {
                        selection.defining = false;
                        selection.define_start = None;
                    }
                }
            }
            if selection.dragging {
                selection.dragging = false;
            }
        }
    }

    // ──── Paint tool interaction (non-select tools) ────
    if tool_state.current_tool != Tool::Select {
        if let Some((px, py)) = canvas.hovered_pixel {
            // Color picker on click
            if tool_state.current_tool == Tool::ColorPicker && response.clicked() {
                tool_state.primary_color = skin.get_pixel(px, py);
                tool_state.current_tool = Tool::Pencil;
            }

            // Pencil/Eraser: on press start stroke, on drag continue
            if matches!(tool_state.current_tool, Tool::Pencil | Tool::Eraser) {
                if response.drag_started_by(egui::PointerButton::Primary) {
                    tool_state.stroking = true;
                    tool_state.stroke_changes.clear();
                    tool_state.last_pos = Some((px, py));
                    let color = tool_state.active_color();
                    let changes = tools::apply_dot(
                        px,
                        py,
                        color,
                        tool_state.brush_size,
                        skin,
                        tool_state.mirror_x,
                        tool_state.mirror_y,
                    );
                    tool_state.stroke_changes.extend(changes);
                    modified = true;
                } else if response.dragged_by(egui::PointerButton::Primary) && tool_state.stroking {
                    if let Some((lx, ly)) = tool_state.last_pos {
                        if (lx, ly) != (px, py) {
                            let color = tool_state.active_color();
                            let changes = tools::apply_line_dots(
                                lx,
                                ly,
                                px,
                                py,
                                color,
                                tool_state.brush_size,
                                skin,
                                tool_state.mirror_x,
                                tool_state.mirror_y,
                            );
                            tool_state.stroke_changes.extend(changes);
                            modified = true;
                        }
                    }
                    tool_state.last_pos = Some((px, py));
                }
            }

            // Bucket fill on click
            if tool_state.current_tool == Tool::Bucket && response.clicked() {
                let color = tool_state.primary_color;
                let changes = tools::apply_bucket(px, py, color, skin);
                if !changes.is_empty() {
                    history.push(HistoryEntry::from_changes(
                        "Bucket fill".to_string(),
                        changes,
                    ));
                    modified = true;
                }
            }

            // Shape tools: on drag start, record start pos; on release, apply shape
            if matches!(
                tool_state.current_tool,
                Tool::Line | Tool::Rectangle | Tool::Ellipse
            ) {
                if response.drag_started_by(egui::PointerButton::Primary) {
                    tool_state.drag_start = Some((px, py));
                    tool_state.stroking = true;
                }

                // Preview shape outline while dragging
                if response.dragged_by(egui::PointerButton::Primary) && tool_state.stroking {
                    if let Some((sx, sy)) = tool_state.drag_start {
                        draw_shape_preview(
                            &painter,
                            canvas_rect,
                            canvas.zoom,
                            sx,
                            sy,
                            px,
                            py,
                            tool_state.current_tool,
                            tool_state.primary_color,
                        );
                    }
                }
            }
        }

        // End stroke on release for pencil/eraser
        if response.drag_stopped() && tool_state.stroking {
            if matches!(tool_state.current_tool, Tool::Pencil | Tool::Eraser) {
                if !tool_state.stroke_changes.is_empty() {
                    let desc = if tool_state.current_tool == Tool::Pencil {
                        "Pencil stroke"
                    } else {
                        "Eraser stroke"
                    };
                    let changes = std::mem::take(&mut tool_state.stroke_changes);
                    history.push(HistoryEntry::from_changes(desc.to_string(), changes));
                }
            }

            // Apply shape on release
            if matches!(
                tool_state.current_tool,
                Tool::Line | Tool::Rectangle | Tool::Ellipse
            ) {
                if let (Some((sx, sy)), Some((px, py))) =
                    (tool_state.drag_start, canvas.hovered_pixel)
                {
                    let color = tool_state.primary_color;
                    let changes = match tool_state.current_tool {
                        Tool::Line => tools::apply_line_dots(
                            sx,
                            sy,
                            px,
                            py,
                            color,
                            tool_state.brush_size,
                            skin,
                            tool_state.mirror_x,
                            tool_state.mirror_y,
                        ),
                        Tool::Rectangle => tools::apply_rect(sx, sy, px, py, color, skin, false),
                        Tool::Ellipse => tools::apply_ellipse(sx, sy, px, py, color, skin, false),
                        _ => Vec::new(),
                    };
                    if !changes.is_empty() {
                        history.push(HistoryEntry::from_changes(
                            format!("{} shape", tool_state.current_tool.name()),
                            changes,
                        ));
                        modified = true;
                    }
                }
            }

            tool_state.stroking = false;
            tool_state.drag_start = None;
            tool_state.last_pos = None;
        }
    }

    if modified {
        if let Some(ref mut handle) = canvas.texture_handle {
            handle.set(skin.to_color_image(), egui::TextureOptions::NEAREST);
        }
    }

    modified
}

pub fn draw_checkerboard(painter: &egui::Painter, rect: egui::Rect, zoom: f32) {
    let check_size = (zoom * 0.5).max(2.0);
    let light = egui::Color32::from_rgb(180, 180, 180);
    let dark = egui::Color32::from_rgb(140, 140, 140);

    let cols = (rect.width() / check_size).ceil() as i32;
    let rows = (rect.height() / check_size).ceil() as i32;

    // Only draw if not too many cells
    if cols * rows > 10000 {
        return;
    }

    for row in 0..rows {
        for col in 0..cols {
            let color = if (row + col) % 2 == 0 { light } else { dark };
            let cell = egui::Rect::from_min_size(
                rect.left_top() + egui::vec2(col as f32 * check_size, row as f32 * check_size),
                egui::vec2(check_size, check_size),
            );
            let cell = cell.intersect(rect);
            if cell.is_positive() {
                painter.rect_filled(cell, 0.0, color);
            }
        }
    }
}

fn draw_grid(painter: &egui::Painter, rect: egui::Rect, zoom: f32) {
    let grid_color = egui::Color32::from_rgba_unmultiplied(255, 255, 255, 30);
    let region_color = egui::Color32::from_rgba_unmultiplied(255, 255, 255, 60);

    for i in 0..=64 {
        let x = rect.left() + i as f32 * zoom;
        let y = rect.top() + i as f32 * zoom;

        // Use thicker lines for region boundaries (every 4 or 8 pixels)
        let stroke = if i % 8 == 0 {
            egui::Stroke::new(1.5, region_color)
        } else if i % 4 == 0 {
            egui::Stroke::new(1.0, region_color)
        } else {
            egui::Stroke::new(0.5, grid_color)
        };

        // Vertical
        painter.line_segment(
            [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
            stroke,
        );
        // Horizontal
        painter.line_segment(
            [egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)],
            stroke,
        );
    }
}

fn draw_region_labels(painter: &egui::Painter, rect: egui::Rect, zoom: f32, show_overlays: bool, is_slim: bool) {
    for lr in uv_map::labeled_rects(is_slim) {
        if lr.is_overlay && !show_overlays {
            continue;
        }

        let region_rect = egui::Rect::from_min_size(
            rect.left_top() + egui::vec2(lr.rect.x as f32 * zoom, lr.rect.y as f32 * zoom),
            egui::vec2(lr.rect.w as f32 * zoom, lr.rect.h as f32 * zoom),
        );

        // Draw region outline
        let alpha = if lr.is_overlay { 120 } else { 180 };
        let outline_color =
            egui::Color32::from_rgba_unmultiplied(lr.color.r(), lr.color.g(), lr.color.b(), alpha);
        painter.rect_stroke(
            region_rect,
            0.0,
            egui::Stroke::new(1.5, outline_color),
            egui::StrokeKind::Middle,
        );

        // Draw label if region is large enough
        if region_rect.width() > 20.0 && region_rect.height() > 12.0 {
            let font = egui::FontId::proportional(9.0_f32.min(zoom * 0.8));
            let text_color = egui::Color32::from_rgba_unmultiplied(
                lr.color.r(),
                lr.color.g(),
                lr.color.b(),
                220,
            );
            painter.text(
                region_rect.center(),
                egui::Align2::CENTER_CENTER,
                &lr.name,
                font,
                text_color,
            );
        }
    }
}

fn draw_shape_preview(
    painter: &egui::Painter,
    rect: egui::Rect,
    zoom: f32,
    x0: u32,
    y0: u32,
    x1: u32,
    y1: u32,
    tool: Tool,
    color: [u8; 4],
) {
    let preview_color = egui::Color32::from_rgba_unmultiplied(color[0], color[1], color[2], 150);
    let stroke = egui::Stroke::new(1.5, preview_color);

    match tool {
        Tool::Line => {
            let p0 =
                rect.left_top() + egui::vec2((x0 as f32 + 0.5) * zoom, (y0 as f32 + 0.5) * zoom);
            let p1 =
                rect.left_top() + egui::vec2((x1 as f32 + 0.5) * zoom, (y1 as f32 + 0.5) * zoom);
            painter.line_segment([p0, p1], stroke);
        }
        Tool::Rectangle => {
            let min_x = x0.min(x1);
            let min_y = y0.min(y1);
            let max_x = x0.max(x1);
            let max_y = y0.max(y1);
            let r = egui::Rect::from_min_max(
                rect.left_top() + egui::vec2(min_x as f32 * zoom, min_y as f32 * zoom),
                rect.left_top() + egui::vec2((max_x + 1) as f32 * zoom, (max_y + 1) as f32 * zoom),
            );
            painter.rect_stroke(r, 0.0, stroke, egui::StrokeKind::Middle);
        }
        Tool::Ellipse => {
            let cx = ((x0 as f32 + x1 as f32) / 2.0 + 0.5) * zoom;
            let cy = ((y0 as f32 + y1 as f32) / 2.0 + 0.5) * zoom;
            let rx = ((x1 as f32 - x0 as f32).abs() / 2.0 + 0.5) * zoom;
            let ry = ((y1 as f32 - y0 as f32).abs() / 2.0 + 0.5) * zoom;
            let center = rect.left_top() + egui::vec2(cx, cy);
            // Approximate ellipse with polygon
            let n = 32;
            let points: Vec<egui::Pos2> = (0..n)
                .map(|i| {
                    let t = i as f32 * std::f32::consts::TAU / n as f32;
                    center + egui::vec2(rx * t.cos(), ry * t.sin())
                })
                .collect();
            painter.add(egui::Shape::closed_line(points, stroke));
        }
        _ => {}
    }
}

/// Draw animated marching-ants selection outline
fn draw_selection_outline(
    painter: &egui::Painter,
    canvas_rect: egui::Rect,
    zoom: f32,
    selection: &Selection,
    ctx: &egui::Context,
) {
    let sx = selection.x as f32 * zoom;
    let sy = selection.y as f32 * zoom;
    let sw = selection.w as f32 * zoom;
    let sh = selection.h as f32 * zoom;

    let sel_rect = egui::Rect::from_min_size(
        canvas_rect.left_top() + egui::vec2(sx, sy),
        egui::vec2(sw, sh),
    );

    // Animate the dash offset for marching ants effect
    let time = ctx.input(|i| i.time) as f32;
    let dash_len = 4.0;
    let offset = (time * 8.0) % (dash_len * 2.0);

    // Draw dashed selection border — two colors for visibility on any background
    let white = egui::Color32::WHITE;
    let black = egui::Color32::BLACK;

    // First pass: solid black background stroke
    painter.rect_stroke(
        sel_rect,
        0.0,
        egui::Stroke::new(1.5, black),
        egui::StrokeKind::Middle,
    );

    // Second pass: dashed white on top (simulated with short line segments)
    let edges = [
        // Top edge
        (sel_rect.left_top(), sel_rect.right_top(), true),
        // Right edge
        (sel_rect.right_top(), sel_rect.right_bottom(), false),
        // Bottom edge
        (sel_rect.left_bottom(), sel_rect.right_bottom(), true),
        // Left edge
        (sel_rect.left_top(), sel_rect.left_bottom(), false),
    ];

    for (start, end, horizontal) in edges {
        let total_len = if horizontal {
            (end.x - start.x).abs()
        } else {
            (end.y - start.y).abs()
        };

        let mut pos = offset;
        let mut drawing = true;
        while pos < total_len {
            let seg_end = (pos + dash_len).min(total_len);
            if drawing {
                let (p0, p1) = if horizontal {
                    (
                        egui::pos2(start.x + pos, start.y),
                        egui::pos2(start.x + seg_end, start.y),
                    )
                } else {
                    (
                        egui::pos2(start.x, start.y + pos),
                        egui::pos2(start.x, start.y + seg_end),
                    )
                };
                painter.line_segment([p0, p1], egui::Stroke::new(1.5, white));
            }
            pos = seg_end;
            drawing = !drawing;
        }
    }

    // Request repaint for animation
    ctx.request_repaint();
}
