use glfw::Context;
use gl::types::*;
use std::ffi::{CString, CStr, c_void};
use std::convert::TryInto;
use std::fs::File;

fn shader_type_name(shader_type: GLenum) -> &'static str {
    match shader_type {
        gl::VERTEX_SHADER => "VERTEX_SHADER",
        gl::FRAGMENT_SHADER => "FRAGMENT_SHADER",
        _ => panic!("Unknown shader type: {}", shader_type)
    }
}

unsafe fn compile_shader(source: &CStr, type_: GLenum) -> GLuint {
    let shader = gl::CreateShader(type_);
    gl::ShaderSource(
        shader,
        1,
        &source.as_ptr(),
        std::ptr::null());
    gl::CompileShader(shader);

    let mut compiled = 0;
    gl::GetShaderiv(shader, gl::COMPILE_STATUS, &mut compiled);

    if compiled != gl::TRUE.into() {
        let mut buffer: [u8; 1024] = [0; 1024];
        let mut length: GLsizei = 0;

        gl::GetShaderInfoLog(
            shader, 
            buffer.len().try_into().unwrap(),
            &mut length, 
            buffer.as_mut_ptr() as *mut i8);

        panic!("Could not compile {} shader: {}",
               shader_type_name(type_),
               std::str::from_utf8(&buffer[0 .. length as usize]).unwrap());
    }

    shader
}

unsafe fn link_program(shaders: &[GLuint]) -> GLuint {
    let program = gl::CreateProgram();
    
    for shader in shaders {
        gl::AttachShader(program, *shader);
    }

    gl::LinkProgram(program);

    let mut linked = 0;
    gl::GetProgramiv(program, gl::LINK_STATUS, &mut linked);

    if linked != gl::TRUE.into() {
        let mut buffer: [u8; 1024] = [0; 1024];
        let mut length: GLsizei = 0;

        gl::GetProgramInfoLog(
            program, 
            buffer.len().try_into().unwrap(),
            &mut length, 
            buffer.as_mut_ptr() as *mut i8);

        panic!("Could not link shader shader: {}",
               std::str::from_utf8(&buffer[0 .. length as usize]).unwrap());
    }

    program
}

fn load_pixels_of_png(file_path: &str) -> (Vec<u8>, i32, i32) {
    let decoder = png::Decoder::new(File::open(file_path).unwrap());
    let (info, mut reader) = decoder.read_info().unwrap();
    let mut pixels = vec![0; info.buffer_size()];
    reader.next_frame(&mut pixels).unwrap();
    // println!("{:?}", info);
    (pixels, info.width.try_into().unwrap(), info.height.try_into().unwrap())
}

unsafe fn load_texture_from_file(file_path: &str) -> GLuint {
    let (pixels, width, height) = load_pixels_of_png(file_path);
    let mut texture = 0;
    gl::GenTextures(1, &mut texture);
    gl::BindTexture(gl::TEXTURE_2D, texture);

    gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST.try_into().unwrap());
    gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::NEAREST_MIPMAP_LINEAR.try_into().unwrap());
    gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE.try_into().unwrap());
    gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE.try_into().unwrap());

    gl::TexImage2D(
        gl::TEXTURE_2D,
        0,
        gl::RGBA.try_into().unwrap(),
        width,
        height,
        0,
        gl::RGB.try_into().unwrap(),
        gl::UNSIGNED_BYTE,
        pixels.as_ptr() as *const c_void,
    );

    gl::GenerateMipmap(gl::TEXTURE_2D);

    texture
}

fn main() {
    let mut glfw = glfw::init(glfw::FAIL_ON_ERRORS).unwrap();
    let (mut window, events) = glfw
        .create_window(800, 600, "Hello this is zozin", glfw::WindowMode::Windowed)
        .expect("Failed to create GLFW window.");

    window.set_key_polling(true);
    window.make_current();

    gl::load_with(|symbol| window.get_proc_address(symbol) as *const _);

    let vertex_shader_source = CString::new(r#"
#version 300 es

precision mediump float;

out vec2 uv;

void main() {
    vec2 position = vec2(
        2.0 * float(gl_VertexID & 1) - 1.0,
        2.0 * float((gl_VertexID >> 1) & 1) - 1.0);
    gl_Position = vec4(
        position,
        0.0,
        1.0);
    uv = (position + vec2(1.0, 1.0)) * 0.5;
}
"#).expect("The unexpectable ZULUL");

    let fragment_shader_source = CString::new(r#"
#version 300 es

precision mediump float;

uniform float time;
uniform sampler2D font;

in vec2 uv;
out vec4 color;

void main() {
    color = texture(font, vec2(uv.x, 1.0 - uv.y));
}
"#).expect("The unexpectable ZULUL");

    let program = unsafe {
        let vertex_shader = compile_shader(&vertex_shader_source, gl::VERTEX_SHADER);
        let fragment_shader = compile_shader(&fragment_shader_source, gl::FRAGMENT_SHADER);
        link_program(&[vertex_shader, fragment_shader])
    };

    let time_uniform = unsafe {
        let name = CString::new("time").expect("The unexpectable");
        gl::GetUniformLocation(program, name.as_ptr())
    };

    let _font_texture = unsafe {
        load_texture_from_file("./charmap-oldschool_white.png")
    };

    while !window.should_close() {
        glfw.poll_events();
        for (_, event) in glfw::flush_messages(&events) {
            match event {
                glfw::WindowEvent::Key(glfw::Key::Escape, _, glfw::Action::Press, _) => {
                    window.set_should_close(true)
                }
                _ => {}
            }
        }

        unsafe {
            gl::ClearColor(0.0, 0.0, 0.0, 1.0);
            gl::Clear(gl::COLOR_BUFFER_BIT);
            gl::UseProgram(program);
            gl::Uniform1f(time_uniform, glfw.get_time() as f32);
            gl::DrawArrays(gl::TRIANGLE_STRIP, 0, 4);
        }

        window.swap_buffers();
    }
}
