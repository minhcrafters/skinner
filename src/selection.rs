/// Selection system: rectangular selection with floating pixel buffer,
/// move, copy/paste, flip, rotate, stretch, and skew transforms.
use crate::history::{PixelChange, SelectionSnapshot};
use crate::skin::SkinTexture;

/// A rectangular selection with an optional floating pixel buffer.
pub struct Selection {
    /// Whether a selection is currently active
    pub active: bool,
    /// Selection rectangle position (top-left corner in skin pixel coords)
    pub x: i32,
    pub y: i32,
    /// Selection dimensions
    pub w: u32,
    pub h: u32,
    /// Floating pixel buffer (RGBA, row-major, w*h entries)
    /// None = selection outline only (not yet lifted), Some = floating/lifted pixels
    pub pixels: Option<Vec<[u8; 4]>>,
    /// The pixels that were under the selection when it was cut (for cancel/restore)
    original_pixels: Option<Vec<[u8; 4]>>,
    /// Original position where cut happened (for cancel/restore)
    original_x: i32,
    original_y: i32,
    /// Clipboard buffer for Ctrl+C / Ctrl+V
    clipboard: Option<ClipboardEntry>,
    /// Whether we're currently drag-defining a new selection
    pub defining: bool,
    /// Start of the definition drag
    pub define_start: Option<(u32, u32)>,
    /// Whether the user is currently dragging the floating selection
    pub dragging: bool,
    /// Offset from mouse to selection origin when drag started
    pub drag_offset: (i32, i32),
}

struct ClipboardEntry {
    pixels: Vec<[u8; 4]>,
    w: u32,
    h: u32,
}

impl Selection {
    pub fn new() -> Self {
        Self {
            active: false,
            x: 0,
            y: 0,
            w: 0,
            h: 0,
            pixels: None,
            original_pixels: None,
            original_x: 0,
            original_y: 0,
            clipboard: None,
            defining: false,
            define_start: None,
            dragging: false,
            drag_offset: (0, 0),
        }
    }

    /// Check if a pixel coordinate is inside the current selection
    pub fn contains(&self, px: i32, py: i32) -> bool {
        self.active
            && px >= self.x
            && py >= self.y
            && px < self.x + self.w as i32
            && py < self.y + self.h as i32
    }

    /// Take a snapshot of the current floating buffer state for undo/redo.
    /// Returns None if there's no floating selection.
    pub fn snapshot(&self) -> Option<SelectionSnapshot> {
        self.pixels.as_ref().map(|px| SelectionSnapshot {
            pixels: px.clone(),
            w: self.w,
            h: self.h,
            x: self.x,
            y: self.y,
        })
    }

    /// Restore a previously captured snapshot (used by undo/redo).
    pub fn restore_snapshot(&mut self, snap: &SelectionSnapshot) {
        self.active = true;
        self.pixels = Some(snap.pixels.clone());
        self.w = snap.w;
        self.h = snap.h;
        self.x = snap.x;
        self.y = snap.y;
    }

    /// Deactivate the selection without producing pixel changes (used by undo/redo).
    pub fn deactivate(&mut self) {
        self.active = false;
        self.pixels = None;
        self.original_pixels = None;
        self.defining = false;
        self.define_start = None;
        self.dragging = false;
    }

    /// Create a selection from a drag rectangle and cut pixels from the skin.
    /// Returns pixel changes (the area being cleared to transparent).
    pub fn select_and_cut(
        &mut self,
        x0: u32,
        y0: u32,
        x1: u32,
        y1: u32,
        skin: &mut SkinTexture,
    ) -> Vec<PixelChange> {
        let min_x = x0.min(x1);
        let max_x = x0.max(x1);
        let min_y = y0.min(y1);
        let max_y = y0.max(y1);
        let w = (max_x - min_x + 1).min(64);
        let h = (max_y - min_y + 1).min(64);

        if w == 0 || h == 0 {
            return Vec::new();
        }

        let mut buffer = Vec::with_capacity((w * h) as usize);
        let mut original = Vec::with_capacity((w * h) as usize);
        let mut changes = Vec::new();

        for dy in 0..h {
            for dx in 0..w {
                let sx = min_x + dx;
                let sy = min_y + dy;
                if sx < 64 && sy < 64 {
                    let pixel = skin.get_pixel(sx, sy);
                    buffer.push(pixel);
                    original.push(pixel);
                    // Clear source to transparent
                    let transparent = [0u8, 0, 0, 0];
                    if pixel != transparent {
                        skin.set_pixel(sx, sy, transparent);
                        changes.push(PixelChange {
                            x: sx,
                            y: sy,
                            old_color: pixel,
                            new_color: transparent,
                        });
                    }
                } else {
                    buffer.push([0, 0, 0, 0]);
                    original.push([0, 0, 0, 0]);
                }
            }
        }

        self.active = true;
        self.x = min_x as i32;
        self.y = min_y as i32;
        self.w = w;
        self.h = h;
        self.pixels = Some(buffer);
        self.original_pixels = Some(original);
        self.original_x = min_x as i32;
        self.original_y = min_y as i32;
        self.defining = false;
        self.define_start = None;

        changes
    }

    /// Commit (stamp) the floating selection back onto the skin.
    /// Returns pixel changes for undo.
    pub fn commit(&mut self, skin: &mut SkinTexture) -> Vec<PixelChange> {
        let mut changes = Vec::new();
        if let Some(ref pixels) = self.pixels {
            for dy in 0..self.h {
                for dx in 0..self.w {
                    let tx = self.x + dx as i32;
                    let ty = self.y + dy as i32;
                    if tx >= 0 && tx < 64 && ty >= 0 && ty < 64 {
                        let idx = (dy * self.w + dx) as usize;
                        let new_color = pixels[idx];
                        // Only stamp non-transparent pixels
                        if new_color[3] > 0 {
                            let old_color = skin.get_pixel(tx as u32, ty as u32);
                            if old_color != new_color {
                                skin.set_pixel(tx as u32, ty as u32, new_color);
                                changes.push(PixelChange {
                                    x: tx as u32,
                                    y: ty as u32,
                                    old_color,
                                    new_color,
                                });
                            }
                        }
                    }
                }
            }
        }
        self.clear();
        changes
    }

    /// Cancel the selection and restore original pixels.
    /// Returns pixel changes for the restoration.
    pub fn cancel(&mut self, skin: &mut SkinTexture) -> Vec<PixelChange> {
        let mut changes = Vec::new();
        if let Some(ref original) = self.original_pixels {
            for dy in 0..self.h {
                for dx in 0..self.w {
                    let tx = self.original_x + dx as i32;
                    let ty = self.original_y + dy as i32;
                    if tx >= 0 && tx < 64 && ty >= 0 && ty < 64 {
                        let idx = (dy * self.w + dx) as usize;
                        let restore_color = original[idx];
                        let current = skin.get_pixel(tx as u32, ty as u32);
                        if current != restore_color {
                            skin.set_pixel(tx as u32, ty as u32, restore_color);
                            changes.push(PixelChange {
                                x: tx as u32,
                                y: ty as u32,
                                old_color: current,
                                new_color: restore_color,
                            });
                        }
                    }
                }
            }
        }
        self.clear();
        changes
    }

    /// Clear all selection state
    fn clear(&mut self) {
        self.active = false;
        self.pixels = None;
        self.original_pixels = None;
        self.defining = false;
        self.define_start = None;
        self.dragging = false;
        self.x = 0;
        self.y = 0;
        self.w = 0;
        self.h = 0;
    }

    /// Deselect: commit if floating, otherwise just clear
    pub fn deselect(&mut self, skin: &mut SkinTexture) -> Vec<PixelChange> {
        if self.pixels.is_some() {
            self.commit(skin)
        } else {
            self.clear();
            Vec::new()
        }
    }

    /// Copy the floating buffer to the clipboard
    pub fn copy_to_clipboard(&mut self) {
        if let Some(ref pixels) = self.pixels {
            self.clipboard = Some(ClipboardEntry {
                pixels: pixels.clone(),
                w: self.w,
                h: self.h,
            });
        }
    }

    /// Paste from clipboard as a new floating selection at (0,0).
    /// Commits any existing selection first.
    pub fn paste_from_clipboard(&mut self, skin: &mut SkinTexture) -> Vec<PixelChange> {
        let mut changes = Vec::new();
        // Commit existing selection first
        if self.active && self.pixels.is_some() {
            changes.extend(self.commit(skin));
        }

        if let Some(ref clip) = self
            .clipboard
            .as_ref()
            .map(|c| (c.pixels.clone(), c.w, c.h))
        {
            let (pixels, w, h) = clip.clone();
            self.active = true;
            self.x = 0;
            self.y = 0;
            self.w = w;
            self.h = h;
            self.pixels = Some(pixels);
            self.original_pixels = None; // pasted, no source to restore
            self.original_x = 0;
            self.original_y = 0;
        }
        changes
    }

    /// Has clipboard content
    pub fn has_clipboard(&self) -> bool {
        self.clipboard.is_some()
    }

    // ──── Transforms ────

    /// Flip the floating buffer horizontally
    pub fn flip_h(&mut self) {
        if let Some(ref mut pixels) = self.pixels {
            let w = self.w as usize;
            let h = self.h as usize;
            for row in 0..h {
                let start = row * w;
                let end = start + w;
                pixels[start..end].reverse();
            }
        }
    }

    /// Flip the floating buffer vertically
    pub fn flip_v(&mut self) {
        if let Some(ref mut pixels) = self.pixels {
            let w = self.w as usize;
            let h = self.h as usize;
            for row in 0..h / 2 {
                let top_start = row * w;
                let bot_start = (h - 1 - row) * w;
                for col in 0..w {
                    pixels.swap(top_start + col, bot_start + col);
                }
            }
        }
    }

    /// Rotate the floating buffer 90° clockwise
    pub fn rotate_cw(&mut self) {
        if let Some(ref pixels) = self.pixels {
            let old_w = self.w as usize;
            let old_h = self.h as usize;
            let new_w = old_h;
            let new_h = old_w;
            let mut rotated = vec![[0u8; 4]; new_w * new_h];
            for y in 0..old_h {
                for x in 0..old_w {
                    let src = y * old_w + x;
                    let dst = x * new_w + (old_h - 1 - y);
                    rotated[dst] = pixels[src];
                }
            }
            self.pixels = Some(rotated);
            self.w = new_w as u32;
            self.h = new_h as u32;
        }
    }

    /// Rotate the floating buffer 90° counter-clockwise
    pub fn rotate_ccw(&mut self) {
        if let Some(ref pixels) = self.pixels {
            let old_w = self.w as usize;
            let old_h = self.h as usize;
            let new_w = old_h;
            let new_h = old_w;
            let mut rotated = vec![[0u8; 4]; new_w * new_h];
            for y in 0..old_h {
                for x in 0..old_w {
                    let src = y * old_w + x;
                    let dst = (old_w - 1 - x) * new_w + y;
                    rotated[dst] = pixels[src];
                }
            }
            self.pixels = Some(rotated);
            self.w = new_w as u32;
            self.h = new_h as u32;
        }
    }

    /// Stretch (resize) the floating buffer to new dimensions using nearest-neighbor sampling.
    /// Clamps to a minimum of 1×1 and maximum of 64×64.
    pub fn stretch(&mut self, new_w: u32, new_h: u32) {
        let new_w = new_w.clamp(1, 64);
        let new_h = new_h.clamp(1, 64);
        if let Some(ref pixels) = self.pixels {
            let old_w = self.w as usize;
            let old_h = self.h as usize;
            let nw = new_w as usize;
            let nh = new_h as usize;
            let mut stretched = vec![[0u8; 4]; nw * nh];
            for y in 0..nh {
                let src_y = (y * old_h / nh).min(old_h - 1);
                for x in 0..nw {
                    let src_x = (x * old_w / nw).min(old_w - 1);
                    stretched[y * nw + x] = pixels[src_y * old_w + src_x];
                }
            }
            self.pixels = Some(stretched);
            self.w = new_w;
            self.h = new_h;
        }
    }

    /// Skew the floating buffer horizontally. Each row is shifted by an amount
    /// proportional to its Y position: row 0 stays put, the last row shifts
    /// by `amount` pixels. Positive = shift right, negative = shift left.
    /// The buffer widens by |amount| to accommodate the shift.
    pub fn skew_h(&mut self, amount: i32) {
        if amount == 0 {
            return;
        }
        if let Some(ref pixels) = self.pixels {
            let old_w = self.w as usize;
            let old_h = self.h as usize;
            if old_h <= 1 {
                return;
            }
            let abs_amount = amount.unsigned_abs() as usize;
            let new_w = (old_w + abs_amount).min(64);
            let mut skewed = vec![[0u8; 4]; new_w * old_h];
            for y in 0..old_h {
                // Linear interpolation of shift: 0 at row 0, amount at last row
                let shift = (amount * y as i32) / (old_h as i32 - 1);
                // Translate so negative shifts anchor from the right
                let offset = if amount < 0 {
                    shift + abs_amount as i32
                } else {
                    shift
                };
                for x in 0..old_w {
                    let nx = x as i32 + offset;
                    if nx >= 0 && (nx as usize) < new_w {
                        skewed[y * new_w + nx as usize] = pixels[y * old_w + x];
                    }
                }
            }
            self.pixels = Some(skewed);
            self.w = new_w as u32;
        }
    }

    /// Skew the floating buffer vertically. Each column is shifted by an amount
    /// proportional to its X position: column 0 stays put, the last column shifts
    /// by `amount` pixels. Positive = shift down, negative = shift up.
    /// The buffer grows taller by |amount| to accommodate the shift.
    pub fn skew_v(&mut self, amount: i32) {
        if amount == 0 {
            return;
        }
        if let Some(ref pixels) = self.pixels {
            let old_w = self.w as usize;
            let old_h = self.h as usize;
            if old_w <= 1 {
                return;
            }
            let abs_amount = amount.unsigned_abs() as usize;
            let new_h = (old_h + abs_amount).min(64);
            let mut skewed = vec![[0u8; 4]; old_w * new_h];
            for x in 0..old_w {
                let shift = (amount * x as i32) / (old_w as i32 - 1);
                let offset = if amount < 0 {
                    shift + abs_amount as i32
                } else {
                    shift
                };
                for y in 0..old_h {
                    let ny = y as i32 + offset;
                    if ny >= 0 && (ny as usize) < new_h {
                        skewed[ny as usize * old_w + x] = pixels[y * old_w + x];
                    }
                }
            }
            self.pixels = Some(skewed);
            self.h = new_h as u32;
        }
    }
}
