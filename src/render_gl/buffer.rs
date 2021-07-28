#![allow(clippy::new_without_default)]

pub type ArrayBuffer = Buffer<{ gl::ARRAY_BUFFER }>;
pub type ElementArrayBuffer = Buffer<{ gl::ELEMENT_ARRAY_BUFFER }>;

pub struct Buffer<const T: gl::types::GLuint> {
    vbo: gl::types::GLuint,
}

impl<const T: gl::types::GLuint> Buffer<T> {
    pub fn new() -> Buffer<T> {
        let mut vbo: gl::types::GLuint = 0;
        unsafe {
            gl::GenBuffers(1, &mut vbo);
        }

        Buffer { vbo }
    }

    pub fn bind(&self) {
        unsafe {
            gl::BindBuffer(T, self.vbo);
        }
    }

    pub fn unbind(&self) {
        unsafe {
            gl::BindBuffer(T, 0);
        }
    }

    pub fn static_draw_data<S>(&self, data: &[S]) {
        self.draw_data(data, gl::STATIC_DRAW);
    }

    pub fn dynamic_draw_data<S>(&self, data: &[S]) {
        self.draw_data(data, gl::DYNAMIC_DRAW);
    }

    fn draw_data<S>(&self, data: &[S], usage: gl::types::GLenum) {
        unsafe {
            gl::BufferData(
                T,
                (data.len() * ::std::mem::size_of::<S>()) as gl::types::GLsizeiptr,
                data.as_ptr() as *const gl::types::GLvoid,
                usage,
            );
        }
    }
}

impl<const T: gl::types::GLuint> Drop for Buffer<T> {
    fn drop(&mut self) {
        self.unbind();
        unsafe {
            gl::DeleteBuffers(1, &self.vbo);
        }
    }
}

pub struct VertexArray {
    vao: gl::types::GLuint,
}

impl VertexArray {
    pub fn new() -> Self {
        let mut vao: gl::types::GLuint = 0;
        unsafe {
            gl::GenVertexArrays(1, &mut vao);
        }

        Self { vao }
    }

    pub fn bind(&self) {
        unsafe {
            gl::BindVertexArray(self.vao);
        }
    }

    pub fn unbind(&self) {
        unsafe {
            gl::BindVertexArray(0);
        }
    }
}

impl Drop for VertexArray {
    fn drop(&mut self) {
        self.unbind();
        unsafe {
            gl::DeleteVertexArrays(1, &self.vao);
        }
    }
}

pub struct Texture {
    texture_id: gl::types::GLuint,
    texture_unit: gl::types::GLuint,
}

impl Texture {
    pub fn new(texture_unit: gl::types::GLenum) -> Self {
        let mut texture_id: gl::types::GLuint = 0;
        unsafe {
            gl::GenTextures(1, &mut texture_id);
        }

        Self {
            texture_id,
            texture_unit,
        }
    }

    pub fn load_texture(
        &self,
        dimensions: (i32, i32),
        pixels: Option<&[u8]>,
        internal_format: gl::types::GLint,
        format: gl::types::GLenum,
        data_type: gl::types::GLenum,
        repeat: bool,
    ) {
        unsafe {
            self.bind();
            let pixels = match pixels {
                Some(slice) => slice.as_ptr() as *const std::ffi::c_void,
                None => std::ptr::null() as *const std::ffi::c_void,
            };

            gl::TexImage2D(
                gl::TEXTURE_2D, // Target
                0,              // Level-of-detail number. 0 for no mip-map
                internal_format,
                dimensions.0,
                dimensions.1,
                0, // Must be zero lol.
                format,
                data_type,
                pixels,
            );

            let param = match repeat {
                true => gl::REPEAT,
                false => gl::CLAMP_TO_BORDER,
            };

            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, param as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, param as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::NEAREST as i32);
            gl::TexParameteri(
                gl::TEXTURE_2D,
                gl::TEXTURE_COMPARE_MODE,
                gl::COMPARE_REF_TO_TEXTURE as i32,
            );
        }
    }

    pub fn set_border_color(&self, border_color: &[f32; 4]) {
        unsafe {
            gl::TexParameterfv(
                gl::TEXTURE_2D,
                gl::TEXTURE_BORDER_COLOR,
                border_color.as_ptr(),
            );
        }
    }

    pub fn bind(&self) {
        unsafe {
            gl::ActiveTexture(self.texture_unit);
            gl::BindTexture(gl::TEXTURE_2D, self.texture_id);
        }
    }

    pub fn bind_to(&self, texture_unit: gl::types::GLenum) {
        unsafe {
            gl::ActiveTexture(texture_unit);
            gl::BindTexture(gl::TEXTURE_2D, self.texture_id);
        }
    }

    pub fn unbind(&self) {
        unsafe {
            gl::ActiveTexture(self.texture_unit);
            gl::BindTexture(gl::TEXTURE_2D, 0);
        }
    }
}

impl Drop for Texture {
    fn drop(&mut self) {
        unsafe { gl::DeleteTextures(1, &self.texture_id) }
    }
}

pub struct FrameBuffer {
    fbo: gl::types::GLuint,
}

impl FrameBuffer {
    pub fn new() -> Self {
        let mut fbo: gl::types::GLuint = 0;
        unsafe {
            gl::GenFramebuffers(1, &mut fbo);
        }

        Self { fbo }
    }

    pub fn set_type(&self, draw_type: gl::types::GLenum, read_type: gl::types::GLenum) {
        self.bind();
        unsafe {
            gl::DrawBuffer(draw_type);
            gl::ReadBuffer(read_type);
        }
    }

    pub fn bind_texture(&self, attachment: gl::types::GLenum, texture: &Texture) {
        self.bind();
        unsafe {
            gl::FramebufferTexture2D(
                gl::FRAMEBUFFER,
                attachment,
                gl::TEXTURE_2D,
                texture.texture_id,
                0,
            );
        }
    }

    pub fn bind(&self) {
        unsafe {
            gl::BindFramebuffer(gl::FRAMEBUFFER, self.fbo);
        }
    }

    pub fn unbind(&self) {
        unsafe {
            gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
        }
    }
}

impl Drop for FrameBuffer {
    fn drop(&mut self) {
        self.unbind();
        unsafe {
            gl::DeleteFramebuffers(1, &self.fbo);
        }
    }
}
