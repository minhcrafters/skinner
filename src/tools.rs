/// Painting tools: pencil, eraser, bucket fill, color picker, line, rect, ellipse.
use crate::history::PixelChange;
use crate::skin::SkinTexture;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tool {
    Pencil,
    Eraser,
    Bucket,
    ColorPicker,
    Line,
    Rectangle,
    Ellipse,
    Select,
}

impl Tool {
    pub fn name(&self) -> &'static str {
        match self {
            Tool::Pencil => "Pencil",
            Tool::Eraser => "Eraser",
            Tool::Bucket => "Bucket",
            Tool::ColorPicker => "Picker",
            Tool::Line => "Line",
            Tool::Rectangle => "Rect",
            Tool::Ellipse => "Ellipse",
            Tool::Select => "Select",
        }
    }

    pub fn shortcut(&self) -> &'static str {
        match self {
            Tool::Pencil => "B",
            Tool::Eraser => "E",
            Tool::Bucket => "G",
            Tool::ColorPicker => "I",
            Tool::Line => "L",
            Tool::Rectangle => "U",
            Tool::Ellipse => "O",
            Tool::Select => "M",
        }
    }
}

pub struct ToolState {
    pub current_tool: Tool,
    pub primary_color: [u8; 4],
    pub secondary_color: [u8; 4],
    pub brush_size: u8,
    pub mirror_x: bool,
    pub mirror_y: bool,
    /// Track drag state for shape tools
    pub drag_start: Option<(u32, u32)>,
    pub last_pos: Option<(u32, u32)>,
    /// Accumulated changes for the current stroke (committed on release)
    pub stroke_changes: Vec<PixelChange>,
    /// Whether we're actively in a stroke
    pub stroking: bool,
}

impl ToolState {
    pub fn new() -> Self {
        Self {
            current_tool: Tool::Pencil,
            primary_color: [0, 0, 0, 255],
            secondary_color: [255, 255, 255, 255],
            brush_size: 1,
            mirror_x: false,
            mirror_y: false,
            drag_start: None,
            last_pos: None,
            stroke_changes: Vec::new(),
            stroking: false,
        }
    }

    pub fn swap_colors(&mut self) {
        std::mem::swap(&mut self.primary_color, &mut self.secondary_color);
    }

    pub fn active_color(&self) -> [u8; 4] {
        match self.current_tool {
            Tool::Eraser => [0, 0, 0, 0],
            _ => self.primary_color,
        }
    }
}

// ──── Tool Operations ────

/// Apply a single pencil/eraser dot, returns pixel changes
pub fn apply_dot(
    x: u32,
    y: u32,
    color: [u8; 4],
    brush_size: u8,
    skin: &mut SkinTexture,
    mirror_x: bool,
    mirror_y: bool,
) -> Vec<PixelChange> {
    let mut changes = Vec::new();
    let r = (brush_size as i32 - 1) / 2;

    for dy in -r..=r {
        for dx in -r..=r {
            let px = x as i32 + dx;
            let py = y as i32 + dy;
            if px >= 0 && px < 64 && py >= 0 && py < 64 {
                let px = px as u32;
                let py = py as u32;
                let old = skin.get_pixel(px, py);
                if old != color {
                    skin.set_pixel(px, py, color);
                    changes.push(PixelChange {
                        x: px,
                        y: py,
                        old_color: old,
                        new_color: color,
                    });
                }
                if mirror_x {
                    let mx = 63 - px;
                    let old = skin.get_pixel(mx, py);
                    if old != color {
                        skin.set_pixel(mx, py, color);
                        changes.push(PixelChange {
                            x: mx,
                            y: py,
                            old_color: old,
                            new_color: color,
                        });
                    }
                }
                if mirror_y {
                    let my = 63 - py;
                    let old = skin.get_pixel(px, my);
                    if old != color {
                        skin.set_pixel(px, my, color);
                        changes.push(PixelChange {
                            x: px,
                            y: my,
                            old_color: old,
                            new_color: color,
                        });
                    }
                }
                if mirror_x && mirror_y {
                    let mx = 63 - px;
                    let my = 63 - py;
                    let old = skin.get_pixel(mx, my);
                    if old != color {
                        skin.set_pixel(mx, my, color);
                        changes.push(PixelChange {
                            x: mx,
                            y: my,
                            old_color: old,
                            new_color: color,
                        });
                    }
                }
            }
        }
    }
    changes
}

/// Draw a line of dots between two points (Bresenham's algorithm)
pub fn apply_line_dots(
    x0: u32,
    y0: u32,
    x1: u32,
    y1: u32,
    color: [u8; 4],
    brush_size: u8,
    skin: &mut SkinTexture,
    mirror_x: bool,
    mirror_y: bool,
) -> Vec<PixelChange> {
    let mut changes = Vec::new();
    let points = bresenham_line(x0 as i32, y0 as i32, x1 as i32, y1 as i32);
    for (px, py) in points {
        if px >= 0 && px < 64 && py >= 0 && py < 64 {
            changes.extend(apply_dot(
                px as u32, py as u32, color, brush_size, skin, mirror_x, mirror_y,
            ));
        }
    }
    changes
}

/// Flood fill from a starting point
pub fn apply_bucket(
    x: u32,
    y: u32,
    fill_color: [u8; 4],
    skin: &mut SkinTexture,
) -> Vec<PixelChange> {
    let target = skin.get_pixel(x, y);
    if target == fill_color {
        return Vec::new();
    }

    let mut changes = Vec::new();
    let mut stack = vec![(x, y)];
    let mut visited = vec![false; 64 * 64];

    while let Some((cx, cy)) = stack.pop() {
        if cx >= 64 || cy >= 64 {
            continue;
        }
        let idx = (cy * 64 + cx) as usize;
        if visited[idx] {
            continue;
        }
        if skin.get_pixel(cx, cy) != target {
            continue;
        }
        visited[idx] = true;

        let old = skin.get_pixel(cx, cy);
        skin.set_pixel(cx, cy, fill_color);
        changes.push(PixelChange {
            x: cx,
            y: cy,
            old_color: old,
            new_color: fill_color,
        });

        if cx > 0 {
            stack.push((cx - 1, cy));
        }
        if cx < 63 {
            stack.push((cx + 1, cy));
        }
        if cy > 0 {
            stack.push((cx, cy - 1));
        }
        if cy < 63 {
            stack.push((cx, cy + 1));
        }
    }
    changes
}

/// Draw a filled rectangle
pub fn apply_rect(
    x0: u32,
    y0: u32,
    x1: u32,
    y1: u32,
    color: [u8; 4],
    skin: &mut SkinTexture,
    filled: bool,
) -> Vec<PixelChange> {
    let mut changes = Vec::new();
    let min_x = x0.min(x1);
    let max_x = x0.max(x1);
    let min_y = y0.min(y1);
    let max_y = y0.max(y1);

    for y in min_y..=max_y {
        for x in min_x..=max_x {
            if x < 64 && y < 64 {
                if filled || x == min_x || x == max_x || y == min_y || y == max_y {
                    let old = skin.get_pixel(x, y);
                    if old != color {
                        skin.set_pixel(x, y, color);
                        changes.push(PixelChange {
                            x,
                            y,
                            old_color: old,
                            new_color: color,
                        });
                    }
                }
            }
        }
    }
    changes
}

/// Draw an ellipse outline or filled
pub fn apply_ellipse(
    x0: u32,
    y0: u32,
    x1: u32,
    y1: u32,
    color: [u8; 4],
    skin: &mut SkinTexture,
    filled: bool,
) -> Vec<PixelChange> {
    let mut changes = Vec::new();
    let cx = (x0 as f32 + x1 as f32) / 2.0;
    let cy = (y0 as f32 + y1 as f32) / 2.0;
    let rx = ((x1 as f32 - x0 as f32) / 2.0).abs().max(0.5);
    let ry = ((y1 as f32 - y0 as f32) / 2.0).abs().max(0.5);

    let min_x = x0.min(x1);
    let max_x = x0.max(x1);
    let min_y = y0.min(y1);
    let max_y = y0.max(y1);

    for y in min_y..=max_y {
        for x in min_x..=max_x {
            if x >= 64 || y >= 64 {
                continue;
            }
            let dx = (x as f32 - cx) / rx;
            let dy = (y as f32 - cy) / ry;
            let dist = dx * dx + dy * dy;

            let inside = dist <= 1.0;
            let on_edge = (dist - 1.0).abs() < (1.0 / rx.min(ry)).max(0.5);

            if (filled && inside) || (!filled && on_edge) {
                let old = skin.get_pixel(x, y);
                if old != color {
                    skin.set_pixel(x, y, color);
                    changes.push(PixelChange {
                        x,
                        y,
                        old_color: old,
                        new_color: color,
                    });
                }
            }
        }
    }
    changes
}

// ──── Helpers ────

fn bresenham_line(x0: i32, y0: i32, x1: i32, y1: i32) -> Vec<(i32, i32)> {
    let mut points = Vec::new();
    let dx = (x1 - x0).abs();
    let dy = -(y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;
    let mut x = x0;
    let mut y = y0;

    loop {
        points.push((x, y));
        if x == x1 && y == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x += sx;
        }
        if e2 <= dx {
            err += dx;
            y += sy;
        }
    }
    points
}
