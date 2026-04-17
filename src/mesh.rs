/// 3D mesh generation for the Minecraft player model.
/// Generates box-based geometry with correct UV mapping for each body part.
use crate::uv_map::BodyPartUV;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Vertex {
    pub position: [f32; 3],
    pub uv: [f32; 2],
    pub normal: [f32; 3],
}

impl Vertex {
    pub fn new(pos: [f32; 3], uv: [f32; 2], normal: [f32; 3]) -> Self {
        Self {
            position: pos,
            uv,
            normal,
        }
    }
}

pub struct MeshData {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
}

impl MeshData {
    pub fn new() -> Self {
        Self {
            vertices: Vec::new(),
            indices: Vec::new(),
        }
    }

    /// Append another mesh's data into this one
    pub fn append(&mut self, other: &MeshData) {
        let base = self.vertices.len() as u32;
        self.vertices.extend_from_slice(&other.vertices);
        for &idx in &other.indices {
            self.indices.push(base + idx);
        }
    }

    /// Get vertex data as a flat f32 slice for GL upload
    pub fn vertex_bytes(&self) -> &[u8] {
        unsafe {
            std::slice::from_raw_parts(
                self.vertices.as_ptr() as *const u8,
                self.vertices.len() * std::mem::size_of::<Vertex>(),
            )
        }
    }

    /// Get index data as bytes
    pub fn index_bytes(&self) -> &[u8] {
        unsafe {
            std::slice::from_raw_parts(
                self.indices.as_ptr() as *const u8,
                self.indices.len() * std::mem::size_of::<u32>(),
            )
        }
    }
}

/// Generate a box mesh with the given origin (center), size, and UV mapping.
///
/// The box is axis-aligned with faces:
/// - Front:  +Z normal
/// - Back:   -Z normal
/// - Right:  +X normal (character's left from front view)
/// - Left:   -X normal (character's right from front view)
/// - Top:    +Y normal
/// - Bottom: -Y normal
fn make_box(origin: [f32; 3], size: [f32; 3], uvs: &BodyPartUV) -> MeshData {
    let [ox, oy, oz] = origin;
    let [sx, sy, sz] = [size[0] / 2.0, size[1] / 2.0, size[2] / 2.0];

    let mut mesh = MeshData::new();

    // Each face: 4 vertices + 6 indices (2 triangles)
    // Vertex order per face: bottom-left, bottom-right, top-right, top-left (CCW from outside)

    // Front face (+Z)
    let uv = uvs.front.to_gl_uvs();
    add_face(
        &mut mesh,
        [ox - sx, oy - sy, oz + sz],
        [ox + sx, oy - sy, oz + sz],
        [ox + sx, oy + sy, oz + sz],
        [ox - sx, oy + sy, oz + sz],
        [0.0, 0.0, 1.0],
        uv,
    );

    // Back face (-Z) — vertices wound CCW from outside (looking from -Z)
    let uv = uvs.back.to_gl_uvs();
    add_face(
        &mut mesh,
        [ox + sx, oy - sy, oz - sz],
        [ox - sx, oy - sy, oz - sz],
        [ox - sx, oy + sy, oz - sz],
        [ox + sx, oy + sy, oz - sz],
        [0.0, 0.0, -1.0],
        uv,
    );

    // Right face (+X) — character's left side
    let uv = uvs.left.to_gl_uvs();
    add_face(
        &mut mesh,
        [ox + sx, oy - sy, oz + sz],
        [ox + sx, oy - sy, oz - sz],
        [ox + sx, oy + sy, oz - sz],
        [ox + sx, oy + sy, oz + sz],
        [1.0, 0.0, 0.0],
        uv,
    );

    // Left face (-X) — character's right side
    let uv = uvs.right.to_gl_uvs();
    add_face(
        &mut mesh,
        [ox - sx, oy - sy, oz - sz],
        [ox - sx, oy - sy, oz + sz],
        [ox - sx, oy + sy, oz + sz],
        [ox - sx, oy + sy, oz - sz],
        [-1.0, 0.0, 0.0],
        uv,
    );

    // Top face (+Y)
    let uv = uvs.top.to_gl_uvs();
    add_face(
        &mut mesh,
        [ox - sx, oy + sy, oz + sz],
        [ox + sx, oy + sy, oz + sz],
        [ox + sx, oy + sy, oz - sz],
        [ox - sx, oy + sy, oz - sz],
        [0.0, 1.0, 0.0],
        uv,
    );

    // Bottom face (-Y)
    let uv = uvs.bottom.to_gl_uvs();
    add_face(
        &mut mesh,
        [ox - sx, oy - sy, oz - sz],
        [ox + sx, oy - sy, oz - sz],
        [ox + sx, oy - sy, oz + sz],
        [ox - sx, oy - sy, oz + sz],
        [0.0, -1.0, 0.0],
        uv,
    );

    mesh
}

fn add_face(
    mesh: &mut MeshData,
    bl: [f32; 3],
    br: [f32; 3],
    tr: [f32; 3],
    tl: [f32; 3],
    normal: [f32; 3],
    uvs: [[f32; 2]; 4], // [bl, br, tr, tl]
) {
    let base = mesh.vertices.len() as u32;
    mesh.vertices.push(Vertex::new(bl, uvs[0], normal));
    mesh.vertices.push(Vertex::new(br, uvs[1], normal));
    mesh.vertices.push(Vertex::new(tr, uvs[2], normal));
    mesh.vertices.push(Vertex::new(tl, uvs[3], normal));
    // Two triangles: bl-br-tr and bl-tr-tl
    mesh.indices
        .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
}

// ──── Player Model Generation ────

/// Model centered at origin, total height ~32 units
/// Legs: y = -16 to -4   (12 high)
/// Body: y = -4  to  8   (12 high)
/// Head: y =  8  to  16  (8 high)
/// Arms: y = -4  to  8   (same as body)
use crate::uv_map;

pub struct PlayerModel {
    pub head: MeshData,
    pub body: MeshData,
    pub right_arm: MeshData,
    pub left_arm: MeshData,
    pub right_leg: MeshData,
    pub left_leg: MeshData,
    pub hat: MeshData,
    pub jacket: MeshData,
    pub right_sleeve: MeshData,
    pub left_sleeve: MeshData,
    pub right_pant: MeshData,
    pub left_pant: MeshData,
}

/// Visibility settings for body parts
#[derive(PartialEq, Clone)]
pub struct PartVisibility {
    pub head: bool,
    pub body: bool,
    pub right_arm: bool,
    pub left_arm: bool,
    pub right_leg: bool,
    pub left_leg: bool,
    pub hat: bool,
    pub jacket: bool,
    pub right_sleeve: bool,
    pub left_sleeve: bool,
    pub right_pant: bool,
    pub left_pant: bool,
}

impl PartVisibility {
    pub fn all_visible() -> Self {
        Self {
            head: true,
            body: true,
            right_arm: true,
            left_arm: true,
            right_leg: true,
            left_leg: true,
            hat: true,
            jacket: true,
            right_sleeve: true,
            left_sleeve: true,
            right_pant: true,
            left_pant: true,
        }
    }
}

impl PlayerModel {
    pub fn generate(is_slim: bool) -> Self {
        let arm_w = if is_slim { 3.0 } else { 4.0 };
        let arm_x_offset = if is_slim { 5.5 } else { 6.0 };
        let overlay_expand = 0.5; // overlay is slightly larger

        // Base parts
        let right_arm_uv = if is_slim { &uv_map::RIGHT_ARM_SLIM_BASE } else { &uv_map::RIGHT_ARM_BASE };
        let left_arm_uv = if is_slim { &uv_map::LEFT_ARM_SLIM_BASE } else { &uv_map::LEFT_ARM_BASE };

        let head = make_box([0.0, 12.0, 0.0], [8.0, 8.0, 8.0], &uv_map::HEAD_BASE);
        let body = make_box([0.0, 2.0, 0.0], [8.0, 12.0, 4.0], &uv_map::BODY_BASE);
        let right_arm = make_box(
            [-arm_x_offset, 2.0, 0.0],
            [arm_w, 12.0, 4.0],
            right_arm_uv,
        );
        let left_arm = make_box(
            [arm_x_offset, 2.0, 0.0],
            [arm_w, 12.0, 4.0],
            left_arm_uv,
        );
        let right_leg = make_box(
            [-2.0, -10.0, 0.0],
            [4.0, 12.0, 4.0],
            &uv_map::RIGHT_LEG_BASE,
        );
        let left_leg = make_box([2.0, -10.0, 0.0], [4.0, 12.0, 4.0], &uv_map::LEFT_LEG_BASE);

        // Overlay parts (slightly expanded)
        let e = overlay_expand;
        let right_sleeve_uv = if is_slim { &uv_map::RIGHT_ARM_SLIM_OVERLAY } else { &uv_map::RIGHT_ARM_OVERLAY };
        let left_sleeve_uv = if is_slim { &uv_map::LEFT_ARM_SLIM_OVERLAY } else { &uv_map::LEFT_ARM_OVERLAY };

        let hat = make_box(
            [0.0, 12.0, 0.0],
            [8.0 + e * 2.0, 8.0 + e * 2.0, 8.0 + e * 2.0],
            &uv_map::HEAD_OVERLAY,
        );
        let jacket = make_box(
            [0.0, 2.0, 0.0],
            [8.0 + e * 2.0, 12.0 + e * 2.0, 4.0 + e * 2.0],
            &uv_map::BODY_OVERLAY,
        );
        let right_sleeve = make_box(
            [-arm_x_offset, 2.0, 0.0],
            [arm_w + e * 2.0, 12.0 + e * 2.0, 4.0 + e * 2.0],
            right_sleeve_uv,
        );
        let left_sleeve = make_box(
            [arm_x_offset, 2.0, 0.0],
            [arm_w + e * 2.0, 12.0 + e * 2.0, 4.0 + e * 2.0],
            left_sleeve_uv,
        );
        let right_pant = make_box(
            [-2.0, -10.0, 0.0],
            [4.0 + e * 2.0, 12.0 + e * 2.0, 4.0 + e * 2.0],
            &uv_map::RIGHT_LEG_OVERLAY,
        );
        let left_pant = make_box(
            [2.0, -10.0, 0.0],
            [4.0 + e * 2.0, 12.0 + e * 2.0, 4.0 + e * 2.0],
            &uv_map::LEFT_LEG_OVERLAY,
        );

        Self {
            head,
            body,
            right_arm,
            left_arm,
            right_leg,
            left_leg,
            hat,
            jacket,
            right_sleeve,
            left_sleeve,
            right_pant,
            left_pant,
        }
    }

    /// Combine all visible parts into a single mesh for rendering.
    /// Returns (mesh, base_index_count) where base_index_count is the split
    /// point between base and overlay indices for two-pass rendering.
    pub fn combined_mesh(&self, vis: &PartVisibility) -> (MeshData, i32) {
        let mut combined = MeshData::new();

        // Base parts (drawn first with depth write ON)
        if vis.head {
            combined.append(&self.head);
        }
        if vis.body {
            combined.append(&self.body);
        }
        if vis.right_arm {
            combined.append(&self.right_arm);
        }
        if vis.left_arm {
            combined.append(&self.left_arm);
        }
        if vis.right_leg {
            combined.append(&self.right_leg);
        }
        if vis.left_leg {
            combined.append(&self.left_leg);
        }
        let base_index_count = combined.indices.len() as i32;

        // Overlay parts (drawn second)
        if vis.hat {
            combined.append(&self.hat);
        }
        if vis.jacket {
            combined.append(&self.jacket);
        }
        if vis.right_sleeve {
            combined.append(&self.right_sleeve);
        }
        if vis.left_sleeve {
            combined.append(&self.left_sleeve);
        }
        if vis.right_pant {
            combined.append(&self.right_pant);
        }
        if vis.left_pant {
            combined.append(&self.left_pant);
        }

        (combined, base_index_count)
    }
}
