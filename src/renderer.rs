use eframe::glow;
use glow::HasContext;

use crate::mesh::{PartVisibility, PlayerModel, Vertex};
use crate::skin::SkinModel;

pub struct Renderer3D {
    program: glow::Program,
    vao: glow::VertexArray,
    vbo: glow::Buffer,
    ebo: glow::Buffer,
    texture: glow::Texture,
    index_count: i32,
    base_index_count: i32,
    pending_pixels: Option<Vec<u8>>,
    mesh_dirty: bool,
    model: PlayerModel,
    visibility: PartVisibility,
    skin_model: SkinModel,
}

impl Renderer3D {
    pub fn new(gl: &glow::Context) -> Self {
        let shader_version = "#version 330";

        let vertex_shader_src = r#"
            uniform mat4 u_mvp;
            layout(location = 0) in vec3 a_pos;
            layout(location = 1) in vec2 a_uv;
            layout(location = 2) in vec3 a_normal;
            out vec2 v_uv;
            out vec3 v_normal;
            void main() {
                gl_Position = u_mvp * vec4(a_pos, 1.0);
                v_uv = a_uv;
                v_normal = a_normal;
            }
        "#;

        let fragment_shader_src = r#"
            precision mediump float;
            uniform sampler2D u_tex;
            in vec2 v_uv;
            in vec3 v_normal;
            out vec4 frag_color;
            void main() {
                vec4 c = texture(u_tex, v_uv);
                if (c.a < 0.01) discard;
                vec3 light_dir = normalize(vec3(0.3, 1.0, 0.5));
                float light = 0.45 + 0.55 * max(dot(normalize(v_normal), light_dir), 0.0);
                frag_color = vec4(c.rgb * light, c.a);
            }
        "#;

        unsafe {
            let program = gl.create_program().expect("Cannot create GL program");

            let shaders = [
                (glow::VERTEX_SHADER, vertex_shader_src),
                (glow::FRAGMENT_SHADER, fragment_shader_src),
            ];

            let compiled_shaders: Vec<glow::Shader> = shaders
                .iter()
                .map(|&(shader_type, src)| {
                    let shader = gl.create_shader(shader_type).expect("Cannot create shader");
                    gl.shader_source(shader, &format!("{shader_version}\n{src}"));
                    gl.compile_shader(shader);
                    assert!(
                        gl.get_shader_compile_status(shader),
                        "Shader compile error: {}",
                        gl.get_shader_info_log(shader)
                    );
                    gl.attach_shader(program, shader);
                    shader
                })
                .collect();

            gl.link_program(program);
            assert!(
                gl.get_program_link_status(program),
                "Program link error: {}",
                gl.get_program_info_log(program)
            );

            for shader in compiled_shaders {
                gl.detach_shader(program, shader);
                gl.delete_shader(shader);
            }

            let vao = gl.create_vertex_array().expect("Cannot create VAO");
            let vbo = gl.create_buffer().expect("Cannot create VBO");
            let ebo = gl.create_buffer().expect("Cannot create EBO");

            let texture = gl.create_texture().expect("Cannot create texture");
            gl.bind_texture(glow::TEXTURE_2D, Some(texture));
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MIN_FILTER,
                glow::NEAREST as i32,
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MAG_FILTER,
                glow::NEAREST as i32,
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_WRAP_S,
                glow::CLAMP_TO_EDGE as i32,
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_WRAP_T,
                glow::CLAMP_TO_EDGE as i32,
            );
            let blank = vec![0u8; 64 * 64 * 4];
            gl.tex_image_2d(
                glow::TEXTURE_2D,
                0,
                glow::RGBA as i32,
                64,
                64,
                0,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                glow::PixelUnpackData::Slice(Some(&blank)),
            );
            gl.bind_texture(glow::TEXTURE_2D, None);

            let model = PlayerModel::generate(false);
            let visibility = PartVisibility::all_visible();

            let mut renderer = Self {
                program,
                vao,
                vbo,
                ebo,
                texture,
                index_count: 0,
                base_index_count: 0,
                pending_pixels: None,
                mesh_dirty: true,
                model,
                visibility,
                skin_model: SkinModel::Classic,
            };

            renderer.upload_mesh(gl);

            renderer
        }
    }

    pub fn set_pending_pixels(&mut self, pixels: Vec<u8>) {
        self.pending_pixels = Some(pixels);
    }

    pub fn set_model_type(&mut self, model: SkinModel) {
        if self.skin_model != model {
            self.skin_model = model;
            self.model = PlayerModel::generate(model == SkinModel::Slim);
            self.mesh_dirty = true;
        }
    }

    pub fn set_visibility(&mut self, vis: PartVisibility) {
        if self.visibility != vis {
            self.visibility = vis;
            self.mesh_dirty = true;
        }
    }

    fn upload_mesh(&mut self, gl: &glow::Context) {
        let (combined, base_count) = self.model.combined_mesh(&self.visibility);
        self.index_count = combined.indices.len() as i32;
        self.base_index_count = base_count;

        unsafe {
            gl.bind_vertex_array(Some(self.vao));

            gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.vbo));
            gl.buffer_data_u8_slice(
                glow::ARRAY_BUFFER,
                combined.vertex_bytes(),
                glow::STATIC_DRAW,
            );

            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(self.ebo));
            gl.buffer_data_u8_slice(
                glow::ELEMENT_ARRAY_BUFFER,
                combined.index_bytes(),
                glow::STATIC_DRAW,
            );

            let stride = std::mem::size_of::<Vertex>() as i32;

            gl.enable_vertex_attrib_array(0);
            gl.vertex_attrib_pointer_f32(0, 3, glow::FLOAT, false, stride, 0);

            gl.enable_vertex_attrib_array(1);
            gl.vertex_attrib_pointer_f32(1, 2, glow::FLOAT, false, stride, 12);

            gl.enable_vertex_attrib_array(2);
            gl.vertex_attrib_pointer_f32(2, 3, glow::FLOAT, false, stride, 20);

            gl.bind_vertex_array(None);
        }

        self.mesh_dirty = false;
    }

    pub fn paint(
        &mut self,
        gl: &glow::Context,
        mvp: &[f32; 16],
        screen_size: [u32; 2],
        clip_rect: [f32; 4],
    ) {
        unsafe {
            if let Some(pixels) = self.pending_pixels.take() {
                gl.bind_texture(glow::TEXTURE_2D, Some(self.texture));
                gl.tex_sub_image_2d(
                    glow::TEXTURE_2D,
                    0,
                    0,
                    0,
                    64,
                    64,
                    glow::RGBA,
                    glow::UNSIGNED_BYTE,
                    glow::PixelUnpackData::Slice(Some(&pixels)),
                );
                gl.bind_texture(glow::TEXTURE_2D, None);
            }

            if self.mesh_dirty {
                self.upload_mesh(gl);
            }

            if self.index_count == 0 {
                return;
            }

            let prev_blend = gl.is_enabled(glow::BLEND);
            let prev_depth = gl.is_enabled(glow::DEPTH_TEST);
            let prev_cull = gl.is_enabled(glow::CULL_FACE);

            // Set up scissor FIRST so depth clear is limited to our area
            gl.enable(glow::SCISSOR_TEST);
            gl.scissor(
                clip_rect[0] as i32,
                (screen_size[1] as f32 - clip_rect[3]) as i32,
                (clip_rect[2] - clip_rect[0]) as i32,
                (clip_rect[3] - clip_rect[1]) as i32,
            );

            gl.enable(glow::DEPTH_TEST);
            gl.depth_func(glow::LEQUAL);
            gl.depth_mask(true);
            gl.enable(glow::CULL_FACE);
            gl.cull_face(glow::BACK);
            gl.enable(glow::BLEND);
            gl.blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);

            gl.clear(glow::DEPTH_BUFFER_BIT);

            gl.use_program(Some(self.program));

            let mvp_loc = gl.get_uniform_location(self.program, "u_mvp");
            gl.uniform_matrix_4_f32_slice(mvp_loc.as_ref(), false, mvp);

            gl.active_texture(glow::TEXTURE0);
            gl.bind_texture(glow::TEXTURE_2D, Some(self.texture));
            let tex_loc = gl.get_uniform_location(self.program, "u_tex");
            gl.uniform_1_i32(tex_loc.as_ref(), 0);

            gl.bind_vertex_array(Some(self.vao));

            if self.base_index_count > 0 {
                gl.depth_mask(true);
                gl.draw_elements(
                    glow::TRIANGLES,
                    self.base_index_count,
                    glow::UNSIGNED_INT,
                    0,
                );
            }

            let overlay_count = self.index_count - self.base_index_count;
            if overlay_count > 0 {
                let offset_bytes = self.base_index_count as i32 * std::mem::size_of::<u32>() as i32;
                gl.draw_elements(
                    glow::TRIANGLES,
                    overlay_count,
                    glow::UNSIGNED_INT,
                    offset_bytes,
                );
            }

            gl.bind_vertex_array(None);

            gl.disable(glow::SCISSOR_TEST);
            gl.use_program(None);
            gl.bind_texture(glow::TEXTURE_2D, None);

            if !prev_depth {
                gl.disable(glow::DEPTH_TEST);
            }
            if !prev_cull {
                gl.disable(glow::CULL_FACE);
            }
            if !prev_blend {
                gl.disable(glow::BLEND);
            }
        }
    }

    pub fn destroy(&self, gl: &glow::Context) {
        unsafe {
            gl.delete_program(self.program);
            gl.delete_vertex_array(self.vao);
            gl.delete_buffer(self.vbo);
            gl.delete_buffer(self.ebo);
            gl.delete_texture(self.texture);
        }
    }
}
