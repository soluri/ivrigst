//! This module contains [Model], which is where the main logic for the mesh rendering happens.
//! Ideally, shadow- and hatching texture rendering should be refactored to it's own module in the
//! future.

use crate::{
    geometry::intersect_box_and_line,
    render_gl::{
        self,
        buffer::{self, FrameBuffer, Texture},
        data::{self, f32_f32_f32},
        Viewport,
    },
    resources::Resources,
};
use anyhow::{Context, Result};
use nalgebra as na;
use render_gl_derive::VertexAttribPointers;

const MAIN_SHADER_PATH: &str = "shaders/model";
const MAIN_SHADER_NAME: &str = "model";
const SHADOW_SHADER_PATH: &str = "shaders/shadow";
const SHADOW_SHADER_NAME: &str = "shadow";
const HATCHING_SHADER_PATH: &str = "shaders/hatching";
const HATCHING_SHADER_NAME: &str = "hatching";
const HATCHING_FAR_PLANE: f32 = 1000.0;
const SHADOW_WIDTH: gl::types::GLsizei = 2048;
const SHADOW_HEIGHT: gl::types::GLsizei = 2048;
const TEXTURE_UNIT_SHADOW: gl::types::GLenum = gl::TEXTURE0;
const TEXTURE_UNIT_HATCH: gl::types::GLenum = gl::TEXTURE1;

#[derive(Copy, Clone, Debug, VertexAttribPointers)]
#[repr(C, packed)]
pub struct Vertex {
    #[location = 0]
    pub pos: data::f32_f32_f32,
    #[location = 1]
    pub normal: data::f32_f32_f32,
    #[location = 2]
    pub color: data::f32_f32_f32,
}

/// Represents which color channel the distance shading shader should use.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(C)]
pub enum DistanceShadingChannel {
    None = 0,
    Hue = 1,
    Saturation = 2,
    Value = 3,
}

impl Default for DistanceShadingChannel {
    fn default() -> Self {
        DistanceShadingChannel::None
    }
}

impl std::fmt::Display for DistanceShadingChannel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            DistanceShadingChannel::None => write!(f, "None"),
            DistanceShadingChannel::Hue => write!(f, "Hue"),
            DistanceShadingChannel::Saturation => write!(f, "Saturation"),
            DistanceShadingChannel::Value => write!(f, "Value"),
        }
    }
}

/// Represents shader attributes in use.
#[derive(Debug, Clone)]
pub struct Attributes {
    pub projection_matrix: na::Matrix4<f32>,
    pub camera_position: na::Vector3<f32>,
    pub light_position: na::Vector3<f32>,
    pub color: na::Vector3<f32>,
    pub model_size: f32,
    pub distance_shading_power: f32,
    pub toon_factor: f32,
    pub distance_shading_channel: DistanceShadingChannel,
    pub shadow_intensity: f32,
    pub shadows_follow: bool,
    pub shadows_orbit_radius: f32,
    pub elapsed: f32,
    pub vertex_color_mix: f32,
    pub hatching_depth: f32,
    pub hatching_frequency: u32,
    pub hatching_steps: u32,
    pub hatching_intensity: f32,
    pub replace_shadows_with_hatching: bool,
}

impl Default for Attributes {
    fn default() -> Self {
        Self {
            projection_matrix: Default::default(),
            camera_position: Default::default(),
            light_position: na::Vector3::new(0.45, 0.25, 0.6),
            color: na::Vector3::new(1.0, 0.56, 0.72),
            model_size: Default::default(),
            distance_shading_power: 0.4,
            toon_factor: 0.7,
            distance_shading_channel: DistanceShadingChannel::None,
            shadow_intensity: 0.6,
            shadows_follow: false,
            shadows_orbit_radius: 25.0,
            elapsed: 0.0,
            vertex_color_mix: 1.0,
            hatching_depth: 1.0,
            hatching_steps: 150,
            hatching_frequency: 4,
            hatching_intensity: 0.5,
            replace_shadows_with_hatching: true,
        }
    }
}

/// [Model] is where the main logic for the mesh rendering happens. Ideally, shadow- and hatching
/// texture rendering should be refactored to it's own module in the future.
pub struct Model {
    program: render_gl::Program,
    shadow_program: render_gl::Program,
    hatching_program: render_gl::Program,
    vao: buffer::VertexArray,
    _vbo: buffer::ArrayBuffer,
    ibo: buffer::ElementArrayBuffer,
    indices: i32,
    size: na::Vector3<f32>,
    attributes: Attributes,
    depth_map: Texture,
    depth_map_fbo: FrameBuffer,
    hatch_map: Texture,
    hatch_map_fbo: FrameBuffer,
}

impl Model {
    /// Set up [Model], compiling shaders, initializing buffers, and parsing a model.
    pub fn new(res: &Resources, filename: &str) -> Result<Self> {
        // set up shader program
        let program = render_gl::Program::from_res(res, MAIN_SHADER_PATH)?;

        let model = res.load_model(filename).context("Failed to load model.")?;

        let mut min = na::Vector3::from_element(f32::MAX);
        let mut max = na::Vector3::from_element(f32::MIN);
        for pos in model.positions.chunks_exact(3) {
            min[0] = min[0].min(pos[0]);
            max[0] = max[0].max(pos[0]);
            min[1] = min[1].min(pos[1]);
            max[1] = max[1].max(pos[1]);
            min[2] = min[2].min(pos[2]);
            max[2] = max[2].max(pos[2]);
        }
        let center = min + (max - min) / 2.0;
        let model_size = (max - min).magnitude();

        let vertices: Vec<Vertex> = model
            .positions
            .chunks_exact(3)
            .zip(model.normals.chunks_exact(3))
            .zip(model.vertex_color.chunks_exact(3))
            .map(|((p, n), c)| {
                (
                    f32_f32_f32::from((p[0] - center[0], p[1] - center[1], p[2] - center[2])),
                    f32_f32_f32::from((n[0], n[1], n[2])),
                    f32_f32_f32::from((c[0], c[1], c[2])),
                )
            })
            .map(|(pos, normal, color)| Vertex { pos, normal, color })
            .collect();
        let vbo = buffer::ArrayBuffer::new();
        vbo.bind();
        vbo.static_draw_data(&vertices);

        // set up vertex array object
        let vao = buffer::VertexArray::new();
        vao.bind();
        Vertex::vertex_attrib_pointers();

        // indices buffer
        let ibo = buffer::ElementArrayBuffer::new();
        ibo.bind();
        ibo.static_draw_data(&model.indices);
        ibo.unbind();
        vbo.unbind();
        vao.unbind();

        // Shadowstuff
        let shadow_program = render_gl::Program::from_res(res, SHADOW_SHADER_PATH)?;
        shadow_program.set_used();

        let depth_map = Texture::new(TEXTURE_UNIT_SHADOW);
        depth_map.load_texture(
            (SHADOW_WIDTH, SHADOW_HEIGHT),
            None,
            gl::DEPTH_COMPONENT as gl::types::GLint,
            gl::DEPTH_COMPONENT,
            gl::FLOAT,
            false,
        );
        depth_map.set_border_color(&[1.0, 1.0, 1.0, 1.0]);

        let depth_map_fbo = FrameBuffer::new();
        depth_map_fbo.bind();
        depth_map_fbo.set_type(gl::NONE, gl::NONE);
        depth_map_fbo.bind_texture(gl::DEPTH_ATTACHMENT, &depth_map);
        depth_map_fbo.unbind();

        let attributes = Attributes {
            model_size,
            ..Default::default()
        };

        let hatching_program = render_gl::Program::from_res(res, HATCHING_SHADER_PATH)?;
        let hatch_map = Texture::new(TEXTURE_UNIT_HATCH);
        hatch_map.load_texture(
            (SHADOW_WIDTH, SHADOW_HEIGHT),
            None,
            gl::DEPTH_COMPONENT as gl::types::GLint,
            gl::DEPTH_COMPONENT,
            gl::FLOAT,
            false,
        );
        hatch_map.set_border_color(&[1.0, 1.0, 1.0, 1.0]);

        let hatch_map_fbo = FrameBuffer::new();
        hatch_map_fbo.bind();
        hatch_map_fbo.set_type(gl::NONE, gl::NONE);
        hatch_map_fbo.bind_texture(gl::DEPTH_ATTACHMENT, &hatch_map);
        hatch_map_fbo.unbind();

        let value = Self {
            program,
            shadow_program,
            hatching_program,
            _vbo: vbo,
            vao,
            ibo,
            indices: model.indices.len() as i32,
            size: max - min,
            attributes,
            depth_map,
            depth_map_fbo,
            hatch_map,
            hatch_map_fbo,
        };
        value.reset_all_attributes();
        Ok(value)
    }

    /// Get the shader attributes.
    pub fn get_attributes(&self) -> &Attributes {
        &self.attributes
    }

    /// Get the hatching texture.
    pub fn get_hatch_texture(&self) -> &Texture {
        &self.hatch_map
    }

    /// Get the shadow texture.
    pub fn get_shadow_texture(&self) -> &Texture {
        &self.depth_map
    }

    /// Compares given [Attributes] struct to the currently applied attributes and updated any
    /// changed values in the shader.
    pub fn set_attributes(&mut self, new: Attributes) {
        let old = &self.attributes;
        self.program.set_used();
        // Safety: data passed to buffers must be of appropriate type and size.
        unsafe {
            if new.projection_matrix != old.projection_matrix {
                self.program
                    .set_uniform_matrix4("projection_matrix", &new.projection_matrix);
                self.hatching_program.set_used();
                self.hatching_program
                    .set_uniform_matrix4("projection_matrix", &new.projection_matrix);
                self.program.set_used();
            }
            if new.camera_position != old.camera_position {
                self.program
                    .set_uniform_3f_na("camera_position", new.camera_position);
            }
            if new.color != old.color {
                self.program.set_uniform_3f_na("color", new.color);
            }
            if (new.model_size - old.model_size).abs() < f32::EPSILON {
                self.program.set_uniform_f("model_size", new.model_size);
            }
            if (new.distance_shading_power - old.distance_shading_power).abs() < f32::EPSILON {
                self.program
                    .set_uniform_f("distance_shading_power", new.distance_shading_power);
            }
            if (new.toon_factor - old.toon_factor).abs() < f32::EPSILON {
                self.program.set_uniform_f("toon_factor", new.toon_factor);
            }
            if new.distance_shading_channel != old.distance_shading_channel {
                self.program.set_uniform_ui(
                    "distance_shading_channel",
                    new.distance_shading_channel as u32,
                )
            }
            if (new.shadow_intensity - old.shadow_intensity).abs() < f32::EPSILON {
                self.program
                    .set_uniform_f("shadow_intensity", new.shadow_intensity)
            }
            if (new.vertex_color_mix - old.vertex_color_mix).abs() < f32::EPSILON {
                self.program
                    .set_uniform_f("vertex_color_mix", new.vertex_color_mix)
            }
            if (new.hatching_intensity - old.hatching_intensity).abs() < f32::EPSILON {
                self.program
                    .set_uniform_f("hatching_intensity", new.hatching_intensity)
            }
            if new.hatching_frequency != old.hatching_frequency {
                self.program
                    .set_uniform_ui("hatching_frequency", new.hatching_frequency)
            }
            if new.replace_shadows_with_hatching != old.replace_shadows_with_hatching {
                self.program.set_uniform_ui(
                    "replace_shadows_with_hatching",
                    new.replace_shadows_with_hatching as u32,
                )
            }
        }
        self.program.unset_used();
        self.attributes = new;
    }

    /// Resets all shader attributes to the defaults.
    pub fn reset_all_attributes(&self) {
        self.program.set_used();
        let att = &self.attributes;
        // Safety: data passed to buffers must be of appropriate type and size.
        unsafe {
            self.program
                .set_uniform_matrix4("projection_matrix", &att.projection_matrix);

            self.program
                .set_uniform_3f_na("camera_position", att.camera_position);
            self.program.set_uniform_3f_na("color", att.color);
            self.program.set_uniform_f("model_size", att.model_size);
            self.program
                .set_uniform_f("distance_shading_power", att.distance_shading_power);
            self.program.set_uniform_f("toon_factor", att.toon_factor);
            self.program.set_uniform_ui(
                "distance_shading_channel",
                att.distance_shading_channel as u32,
            );
            self.program
                .set_uniform_f("shadow_intensity", att.shadow_intensity);
            self.program
                .set_uniform_f("vertex_color_mix", att.vertex_color_mix);
            self.program
                .set_uniform_f("hatching_intensity", att.hatching_intensity);
            self.program
                .set_uniform_ui("hatching_frequency", att.hatching_frequency);
            self.program.set_uniform_ui(
                "replace_shadows_with_hatching",
                att.replace_shadows_with_hatching as u32,
            );
        }
        self.program.unset_used();
    }

    /// Gets the bounding box size of the loaded model.
    pub fn get_size(&self) -> &na::Vector3<f32> {
        &self.size
    }

    /// The main rendering function for the program.
    pub fn render(&self, viewport: &Viewport) {
        // Safety: This is a non-stop stream of OpenGL calls. Ultimately, without a safe wrappe
        // around OpenGL (which even `glium` eventually had to give up on), this will likely never
        // be entirely safe.
        unsafe {
            let (light_vector, light_space_matrix) = self.render_shadowmap();
            let hatch_space_matrix = self.render_hatchmap(viewport);

            // Calculate distance shading planes
            let cam = self.attributes.camera_position;
            let mut intersections = intersect_box_and_line(cam, self.size).to_vec();
            intersections.sort_unstable_by(|&a, &b| {
                (cam - a)
                    .norm()
                    .partial_cmp(&(cam - b).norm())
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            let closest = intersections[0];
            let furthest = intersections[1];
            self.program.set_used();
            self.program.set_uniform_3f(
                "distance_shading_closest",
                (closest.x, closest.y, closest.z),
            );
            self.program.set_uniform_3f(
                "distance_shading_furthest",
                (furthest.x, furthest.y, furthest.z),
            );

            // Main render of model using shadows.
            self.program
                .set_uniform_matrix4("light_space_matrix", &light_space_matrix);
            self.program
                .set_uniform_matrix4("light_space_matrix", &light_space_matrix);
            self.program
                .set_uniform_matrix4("hatch_space_matrix", &hatch_space_matrix);
            self.program.set_uniform_3f(
                "light_vector",
                (light_vector[0], light_vector[1], light_vector[2]),
            );
            self.program
                .set_uniform_f("hatching_far_plane", HATCHING_FAR_PLANE);
            gl::Enable(gl::CULL_FACE);
            gl::CullFace(gl::BACK);
            viewport.set_used();
            self.vao.bind();
            self.ibo.bind();
            self.depth_map.bind_to(gl::TEXTURE0);
            self.hatch_map.bind_to(gl::TEXTURE0 + 1);
            if self.attributes.replace_shadows_with_hatching {
                self.hatch_map
                    .set_texture_compare_mode(gl::COMPARE_REF_TO_TEXTURE);
            } else {
                self.hatch_map.set_texture_compare_mode(gl::NONE);
            }
            gl::DrawElements(
                gl::TRIANGLES,
                self.indices,
                gl::UNSIGNED_INT,
                std::ptr::null::<std::ffi::c_void>(),
            );
        }
        self.hatch_map.unbind();
        self.depth_map.unbind();
        self.ibo.unbind();
        self.vao.unbind();
    }

    /// Renders the shadowmap to the shadows framebuffer.
    ///
    /// ### Safety
    ///
    /// Requires buffers and data in the struct to be appropriately set.
    /// This function should only be called from [Model::render].
    unsafe fn render_shadowmap(&self) -> (na::OPoint<f32, na::Const<3>>, na::Matrix4<f32>) {
        gl::Disable(gl::CULL_FACE);
        gl::Disable(gl::BLEND);
        gl::Enable(gl::DEPTH_TEST);
        gl::DepthFunc(gl::LESS);
        self.shadow_program.set_used();
        let near_plane = 1.0;
        let far_plane = 500.0;
        let bound = 250.0;
        let light_projection =
            na::Orthographic3::new(-bound, bound, -bound, bound, near_plane, far_plane);
        let light_pos = match self.attributes.shadows_follow {
            true => self.attributes.camera_position,
            false => self.attributes.light_position,
        };
        let light = light_pos.normalize() * self.attributes.camera_position.magnitude();
        let cycle_speed_ms = 2000.0;
        let degrees =
            (self.attributes.elapsed % cycle_speed_ms) / cycle_speed_ms * std::f32::consts::TAU;
        let axis = na::Unit::new_normalize(light);
        let rotation = na::Matrix4::from_axis_angle(&axis, degrees);
        let horizontal = na::Vector3::new(0.0, 1.0, 0.0).cross(&light);
        let up_vector = horizontal.cross(&light).normalize() * self.attributes.shadows_orbit_radius;
        let light = (rotation * (light + up_vector).to_homogeneous()).xyz();
        let center = na::Point3::new(0.0, 0.0, 0.0);
        let light_view = na::Matrix4::look_at_rh(
            &na::Point3::from(light),
            &center,
            &na::Vector3::new(0.0, 1.0, 0.0),
        );
        let light_vector = center - light;
        let light_space_matrix = light_projection.to_homogeneous() * light_view;
        self.shadow_program
            .set_uniform_matrix4("lightSpaceMatrix", &light_space_matrix);
        gl::Viewport(0, 0, SHADOW_WIDTH, SHADOW_HEIGHT);
        self.depth_map_fbo.bind();
        gl::Clear(gl::DEPTH_BUFFER_BIT);
        self.vao.bind();
        self.ibo.bind();
        gl::DrawElements(
            gl::TRIANGLES,
            self.indices,
            gl::UNSIGNED_INT,
            std::ptr::null::<std::ffi::c_void>(),
        );
        self.depth_map_fbo.unbind();
        (light_vector, light_space_matrix)
    }

    /// Renders the hatchmap to the hatching framebuffer.
    ///
    /// ### Safety
    ///
    /// Requires buffers and data in the struct to be appropriately set.
    /// This function should only be called from [Model::render].
    unsafe fn render_hatchmap(&self, viewport: &Viewport) -> na::Matrix4<f32> {
        self.hatching_program.set_used();
        self.hatching_program
            .set_uniform_f("hatching_depth", self.attributes.hatching_depth);
        self.hatching_program
            .set_uniform_ui("steps", self.attributes.hatching_steps);
        self.hatch_map_fbo.bind();

        let near_plane = 0.1;
        let aspect = viewport.size().0 as f32 / viewport.size().1 as f32;
        let hatch_projection = na::Perspective3::new(
            aspect,
            std::f32::consts::PI / 4.0,
            near_plane,
            HATCHING_FAR_PLANE,
        );
        let hatch_pos = self.attributes.camera_position;
        let center = na::Point3::new(0.0, 0.0, 0.0);
        let hatch_view = na::Matrix4::look_at_rh(
            &na::Point3::from(hatch_pos),
            &center,
            &na::Vector3::new(0.0, 1.0, 0.0),
        );

        let hatch_space_matrix = hatch_projection.to_homogeneous() * hatch_view;
        self.hatching_program
            .set_uniform_matrix4("projection_matrix", &hatch_space_matrix);
        self.hatching_program
            .set_uniform_f("far_plane", HATCHING_FAR_PLANE);

        gl::Disable(gl::CULL_FACE);
        gl::Disable(gl::BLEND);
        gl::Enable(gl::DEPTH_TEST);
        gl::DepthFunc(gl::LESS);
        gl::Viewport(0, 0, SHADOW_WIDTH, SHADOW_HEIGHT);
        gl::Clear(gl::DEPTH_BUFFER_BIT);
        self.vao.bind();
        self.ibo.bind();
        gl::DrawElements(
            gl::TRIANGLES,
            self.indices,
            gl::UNSIGNED_INT,
            std::ptr::null::<std::ffi::c_void>(),
        );
        self.hatch_map_fbo.unbind();
        hatch_space_matrix
    }

    /// Check if any of the shaders have been updated.
    pub fn check_shader_update(&mut self, path: &std::path::Path, res: &Resources) -> bool {
        let path = path.file_stem().map(|p| p.to_string_lossy().to_string());
        if path == Some(MAIN_SHADER_NAME.to_string()) {
            match render_gl::Program::from_res(res, MAIN_SHADER_PATH) {
                Ok(program) => {
                    self.program.unset_used();
                    self.program = program;
                    self.reset_all_attributes();
                    return true;
                }
                Err(e) => eprintln!("Shader reload error: {}", e),
            }
        } else if path == Some(SHADOW_SHADER_NAME.to_string()) {
            match render_gl::Program::from_res(res, SHADOW_SHADER_PATH) {
                Ok(program) => {
                    self.shadow_program.unset_used();
                    self.shadow_program = program;
                    self.reset_all_attributes();
                    return true;
                }
                Err(e) => eprintln!("Shader reload error: {}", e),
            }
        } else if path == Some(HATCHING_SHADER_NAME.to_string()) {
            match render_gl::Program::from_res(res, HATCHING_SHADER_PATH) {
                Ok(program) => {
                    self.hatching_program.unset_used();
                    self.hatching_program = program;
                    self.reset_all_attributes();
                    return true;
                }
                Err(e) => eprintln!("Shader reload error: {}", e),
            }
        }
        false
    }
}
