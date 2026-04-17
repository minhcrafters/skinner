/// Complete UV region definitions for the 64×64 Minecraft skin format.
/// Every body part face maps to a specific rectangle in the skin texture.
use eframe::egui;

/// A rectangular region on the skin texture
#[derive(Clone, Copy, Debug)]
pub struct UVRect {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

impl UVRect {
    pub const fn new(x: u32, y: u32, w: u32, h: u32) -> Self {
        Self { x, y, w, h }
    }

    /// Check if a pixel coordinate falls within this rect
    pub fn contains(&self, px: u32, py: u32) -> bool {
        px >= self.x && px < self.x + self.w && py >= self.y && py < self.y + self.h
    }

    /// Convert to OpenGL UV coordinates.
    /// Texture data is uploaded top-to-bottom, so the first row sits at V=0
    /// and the last row at V=1. No Y-flip needed.
    /// UVs are inset by half a texel to prevent sampling across region
    /// boundaries, which causes visible seams.
    pub fn to_gl_uvs(&self) -> [[f32; 2]; 4] {
        let tex_w = 64.0_f32;
        let tex_h = 64.0_f32;
        let half_px_u = 0.5 / tex_w;
        let half_px_v = 0.5 / tex_h;
        let u0 = self.x as f32 / tex_w + half_px_u;
        let u1 = (self.x + self.w) as f32 / tex_w - half_px_u;
        let v0 = self.y as f32 / tex_h + half_px_v; // top of rect (low V)
        let v1 = (self.y + self.h) as f32 / tex_h - half_px_v; // bottom of rect (high V)
        // Returns [bottom-left, bottom-right, top-right, top-left]
        [[u0, v1], [u1, v1], [u1, v0], [u0, v0]]
    }
}

/// UV regions for all 6 faces of a body part
#[derive(Clone, Copy, Debug)]
pub struct BodyPartUV {
    pub right: UVRect,  // -X face (character's right)
    pub front: UVRect,  // +Z face (character's front)
    pub left: UVRect,   // +X face (character's left)
    pub back: UVRect,   // -Z face (character's back)
    pub top: UVRect,    // +Y face
    pub bottom: UVRect, // -Y face
}

/// A named region for display in the 2D canvas
#[derive(Clone, Debug)]
pub struct LabeledRect {
    pub name: String,
    pub rect: UVRect,
    pub color: egui::Color32,
    pub is_overlay: bool,
}

// ──── Base Layer Regions ────

pub const HEAD_BASE: BodyPartUV = BodyPartUV {
    right: UVRect::new(0, 8, 8, 8),
    front: UVRect::new(8, 8, 8, 8),
    left: UVRect::new(16, 8, 8, 8),
    back: UVRect::new(24, 8, 8, 8),
    top: UVRect::new(8, 0, 8, 8),
    bottom: UVRect::new(16, 0, 8, 8),
};

pub const BODY_BASE: BodyPartUV = BodyPartUV {
    right: UVRect::new(16, 20, 4, 12),
    front: UVRect::new(20, 20, 8, 12),
    left: UVRect::new(28, 20, 4, 12),
    back: UVRect::new(32, 20, 8, 12),
    top: UVRect::new(20, 16, 8, 4),
    bottom: UVRect::new(28, 16, 8, 4),
};

pub const RIGHT_ARM_BASE: BodyPartUV = BodyPartUV {
    right: UVRect::new(40, 20, 4, 12),
    front: UVRect::new(44, 20, 4, 12),
    left: UVRect::new(48, 20, 4, 12),
    back: UVRect::new(52, 20, 4, 12),
    top: UVRect::new(44, 16, 4, 4),
    bottom: UVRect::new(48, 16, 4, 4),
};

pub const LEFT_ARM_BASE: BodyPartUV = BodyPartUV {
    right: UVRect::new(32, 52, 4, 12),
    front: UVRect::new(36, 52, 4, 12),
    left: UVRect::new(40, 52, 4, 12),
    back: UVRect::new(44, 52, 4, 12),
    top: UVRect::new(36, 48, 4, 4),
    bottom: UVRect::new(40, 48, 4, 4),
};

// ──── Slim Arm Base Layers ────
pub const RIGHT_ARM_SLIM_BASE: BodyPartUV = BodyPartUV {
    right: UVRect::new(40, 20, 4, 12),
    front: UVRect::new(44, 20, 3, 12),
    left: UVRect::new(47, 20, 4, 12),
    back: UVRect::new(51, 20, 3, 12),
    top: UVRect::new(44, 16, 3, 4),
    bottom: UVRect::new(47, 16, 3, 4),
};

pub const LEFT_ARM_SLIM_BASE: BodyPartUV = BodyPartUV {
    right: UVRect::new(32, 52, 4, 12),
    front: UVRect::new(36, 52, 3, 12),
    left: UVRect::new(39, 52, 4, 12),
    back: UVRect::new(43, 52, 3, 12),
    top: UVRect::new(36, 48, 3, 4),
    bottom: UVRect::new(39, 48, 3, 4),
};

pub const RIGHT_LEG_BASE: BodyPartUV = BodyPartUV {
    right: UVRect::new(0, 20, 4, 12),
    front: UVRect::new(4, 20, 4, 12),
    left: UVRect::new(8, 20, 4, 12),
    back: UVRect::new(12, 20, 4, 12),
    top: UVRect::new(4, 16, 4, 4),
    bottom: UVRect::new(8, 16, 4, 4),
};

pub const LEFT_LEG_BASE: BodyPartUV = BodyPartUV {
    right: UVRect::new(16, 52, 4, 12),
    front: UVRect::new(20, 52, 4, 12),
    left: UVRect::new(24, 52, 4, 12),
    back: UVRect::new(28, 52, 4, 12),
    top: UVRect::new(20, 48, 4, 4),
    bottom: UVRect::new(24, 48, 4, 4),
};

// ──── Overlay Layer Regions ────

pub const HEAD_OVERLAY: BodyPartUV = BodyPartUV {
    right: UVRect::new(32, 8, 8, 8),
    front: UVRect::new(40, 8, 8, 8),
    left: UVRect::new(48, 8, 8, 8),
    back: UVRect::new(56, 8, 8, 8),
    top: UVRect::new(40, 0, 8, 8),
    bottom: UVRect::new(48, 0, 8, 8),
};

pub const BODY_OVERLAY: BodyPartUV = BodyPartUV {
    right: UVRect::new(16, 36, 4, 12),
    front: UVRect::new(20, 36, 8, 12),
    left: UVRect::new(28, 36, 4, 12),
    back: UVRect::new(32, 36, 8, 12),
    top: UVRect::new(20, 32, 8, 4),
    bottom: UVRect::new(28, 32, 8, 4),
};

pub const RIGHT_ARM_OVERLAY: BodyPartUV = BodyPartUV {
    right: UVRect::new(40, 36, 4, 12),
    front: UVRect::new(44, 36, 4, 12),
    left: UVRect::new(48, 36, 4, 12),
    back: UVRect::new(52, 36, 4, 12),
    top: UVRect::new(44, 32, 4, 4),
    bottom: UVRect::new(48, 32, 4, 4),
};

pub const LEFT_ARM_OVERLAY: BodyPartUV = BodyPartUV {
    right: UVRect::new(48, 52, 4, 12),
    front: UVRect::new(52, 52, 4, 12),
    left: UVRect::new(56, 52, 4, 12),
    back: UVRect::new(60, 52, 4, 12),
    top: UVRect::new(52, 48, 4, 4),
    bottom: UVRect::new(56, 48, 4, 4),
};

// ──── Slim Arm Overlay Layers ────
pub const RIGHT_ARM_SLIM_OVERLAY: BodyPartUV = BodyPartUV {
    right: UVRect::new(40, 36, 4, 12),
    front: UVRect::new(44, 36, 3, 12),
    left: UVRect::new(47, 36, 4, 12),
    back: UVRect::new(51, 36, 3, 12),
    top: UVRect::new(44, 32, 3, 4),
    bottom: UVRect::new(47, 32, 3, 4),
};

pub const LEFT_ARM_SLIM_OVERLAY: BodyPartUV = BodyPartUV {
    right: UVRect::new(48, 52, 4, 12),
    front: UVRect::new(52, 52, 3, 12),
    left: UVRect::new(55, 52, 4, 12),
    back: UVRect::new(59, 52, 3, 12),
    top: UVRect::new(52, 48, 3, 4),
    bottom: UVRect::new(55, 48, 3, 4),
};

pub const RIGHT_LEG_OVERLAY: BodyPartUV = BodyPartUV {
    right: UVRect::new(0, 36, 4, 12),
    front: UVRect::new(4, 36, 4, 12),
    left: UVRect::new(8, 36, 4, 12),
    back: UVRect::new(12, 36, 4, 12),
    top: UVRect::new(4, 32, 4, 4),
    bottom: UVRect::new(8, 32, 4, 4),
};

pub const LEFT_LEG_OVERLAY: BodyPartUV = BodyPartUV {
    right: UVRect::new(0, 52, 4, 12),
    front: UVRect::new(4, 52, 4, 12),
    left: UVRect::new(8, 52, 4, 12),
    back: UVRect::new(12, 52, 4, 12),
    top: UVRect::new(4, 48, 4, 4),
    bottom: UVRect::new(8, 48, 4, 4),
};

/// Get labeled rectangles for drawing region outlines on the 2D canvas
pub fn labeled_rects(is_slim: bool) -> Vec<LabeledRect> {
    let mut rects = Vec::new();
    let base_colors = [
        ("Head", egui::Color32::from_rgb(255, 100, 100)),
        ("Body", egui::Color32::from_rgb(100, 255, 100)),
        ("R.Arm", egui::Color32::from_rgb(100, 100, 255)),
        ("L.Arm", egui::Color32::from_rgb(255, 255, 100)),
        ("R.Leg", egui::Color32::from_rgb(255, 100, 255)),
        ("L.Leg", egui::Color32::from_rgb(100, 255, 255)),
    ];
    let overlay_colors = [
        ("Hat", egui::Color32::from_rgb(200, 80, 80)),
        ("Jacket", egui::Color32::from_rgb(80, 200, 80)),
        ("R.Sleeve", egui::Color32::from_rgb(80, 80, 200)),
        ("L.Sleeve", egui::Color32::from_rgb(200, 200, 80)),
        ("R.Pant", egui::Color32::from_rgb(200, 80, 200)),
        ("L.Pant", egui::Color32::from_rgb(80, 200, 200)),
    ];

    let base_parts = [
        HEAD_BASE,
        BODY_BASE,
        if is_slim { RIGHT_ARM_SLIM_BASE } else { RIGHT_ARM_BASE },
        if is_slim { LEFT_ARM_SLIM_BASE } else { LEFT_ARM_BASE },
        RIGHT_LEG_BASE,
        LEFT_LEG_BASE,
    ];
    let overlay_parts = [
        HEAD_OVERLAY,
        BODY_OVERLAY,
        if is_slim { RIGHT_ARM_SLIM_OVERLAY } else { RIGHT_ARM_OVERLAY },
        if is_slim { LEFT_ARM_SLIM_OVERLAY } else { LEFT_ARM_OVERLAY },
        RIGHT_LEG_OVERLAY,
        LEFT_LEG_OVERLAY,
    ];

    for (i, part) in base_parts.iter().enumerate() {
        let (name, color) = base_colors[i];
        for (face_name, face_rect) in faces_of(part) {
            rects.push(LabeledRect {
                name: format!("{name} {face_name}"),
                rect: face_rect,
                color,
                is_overlay: false,
            });
        }
    }

    for (i, part) in overlay_parts.iter().enumerate() {
        let (name, color) = overlay_colors[i];
        for (face_name, face_rect) in faces_of(part) {
            rects.push(LabeledRect {
                name: format!("{name} {face_name}"),
                rect: face_rect,
                color,
                is_overlay: true,
            });
        }
    }

    rects
}

fn faces_of(part: &BodyPartUV) -> Vec<(&'static str, UVRect)> {
    vec![
        ("R", part.right),
        ("F", part.front),
        ("L", part.left),
        ("Bk", part.back),
        ("T", part.top),
        ("Bt", part.bottom),
    ]
}

/// Find which region a pixel belongs to, returns region name
pub fn region_at_pixel(x: u32, y: u32, is_slim: bool) -> Option<String> {
    for lr in labeled_rects(is_slim) {
        // ... (check overlay layers first, or standard list order handles it)
        if lr.rect.contains(x, y) {
            return Some(lr.name);
        }
    }
    None
}
