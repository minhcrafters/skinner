use crate::canvas::draw_checkerboard;
use eframe::egui;
use std::path::Path;

pub struct ReferenceImage {
    pub title: String,
    pub open: bool,
    pub texture: Option<egui::TextureHandle>,
    pub rgb_data: image::RgbaImage,
    pub zoom: f32,
    pub pan_offset: egui::Vec2,
}

impl ReferenceImage {
    pub fn load(path: &Path) -> Result<Self, String> {
        let img = image::open(path).map_err(|e| e.to_string())?.into_rgba8();
        let title = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        Ok(Self {
            title,
            open: true,
            texture: None,
            rgb_data: img,
            zoom: 1.0,
            pan_offset: egui::Vec2::ZERO,
        })
    }

    pub fn show_window(&mut self, ctx: &egui::Context) -> Option<[u8; 4]> {
        let mut picked_color = None;
        let mut open = self.open;

        egui::Window::new(&self.title)
            .open(&mut open)
            .show(ctx, |ui| {
                if self.texture.is_none() {
                    let size = [
                        self.rgb_data.width() as usize,
                        self.rgb_data.height() as usize,
                    ];
                    let pixels = self.rgb_data.as_flat_samples();
                    let color_image =
                        egui::ColorImage::from_rgba_unmultiplied(size, pixels.as_slice());
                    self.texture = Some(ctx.load_texture(
                        &self.title,
                        color_image,
                        egui::TextureOptions::NEAREST,
                    ));
                }

                if let Some(tex) = &self.texture {
                    let available = ui.available_size();

                    let (response, painter) =
                        ui.allocate_painter(available, egui::Sense::click_and_drag());

                    if response.dragged_by(egui::PointerButton::Middle)
                        || response.dragged_by(egui::PointerButton::Secondary)
                    {
                        self.pan_offset += response.drag_delta();
                    }

                    let scroll = ui.input(|i| i.smooth_scroll_delta.y);
                    if response.hovered() && scroll.abs() > 0.0 {
                        let old_zoom = self.zoom;
                        let factor = (1.0 + scroll * 0.002).clamp(0.9, 1.1);
                        self.zoom = (self.zoom * factor).clamp(0.1, 50.0);

                        if let Some(pointer) = response.hover_pos() {
                            let canvas_origin = response.rect.left_top() + self.pan_offset;
                            let cursor_in_canvas = pointer - canvas_origin;
                            let scale = self.zoom / old_zoom;
                            let new_cursor_in_canvas = cursor_in_canvas * scale;
                            self.pan_offset -= new_cursor_in_canvas - cursor_in_canvas;
                        }
                    }

                    // The origin point after panning
                    let canvas_origin = response.rect.left_top() + self.pan_offset;
                    let canvas_pixel_size = egui::vec2(
                        self.rgb_data.width() as f32 * self.zoom,
                        self.rgb_data.height() as f32 * self.zoom,
                    );
                    let canvas_rect = egui::Rect::from_min_size(canvas_origin, canvas_pixel_size);

                    draw_checkerboard(&painter, canvas_rect, self.zoom.max(1.0));

                    painter.image(
                        tex.id(),
                        canvas_rect,
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        egui::Color32::WHITE,
                    );

                    if response.dragged_by(egui::PointerButton::Primary) || response.clicked() {
                        if let Some(pos) = response.interact_pointer_pos().or(response.hover_pos())
                        {
                            let local = pos - canvas_rect.left_top();
                            let rel_x = local.x / canvas_rect.width();
                            let rel_y = local.y / canvas_rect.height();

                            if rel_x >= 0.0 && rel_x <= 1.0 && rel_y >= 0.0 && rel_y <= 1.0 {
                                let px = (rel_x * self.rgb_data.width() as f32).floor() as u32;
                                let py = (rel_y * self.rgb_data.height() as f32).floor() as u32;

                                let px = px.min(self.rgb_data.width().saturating_sub(1));
                                let py = py.min(self.rgb_data.height().saturating_sub(1));

                                let pixel = self.rgb_data.get_pixel(px, py);
                                picked_color = Some([pixel[0], pixel[1], pixel[2], pixel[3]]);
                            }
                        }
                    }
                }
            });

        self.open = open;
        picked_color
    }
}
