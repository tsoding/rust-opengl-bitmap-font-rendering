use glfw::Context;
use gl::types::*;
use std::ffi::{CString, CStr, c_void};
use std::convert::TryInto;
use std::fs::File;
use std::cmp::min;

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

    gl::UseProgram(program);

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

unsafe fn copy_str_to_buffer(string_buffer_data: &mut [i32], string_buffer_id: GLuint, payload: &str) {
    for (dst, src) in string_buffer_data.iter_mut().zip(payload.bytes()) {
        *dst = src as i32;
    }
    gl::BindBuffer(gl::ARRAY_BUFFER, string_buffer_id);
    let size = std::mem::size_of_val(&string_buffer_data[0]) * min(string_buffer_data.len(), payload.len());
    gl::BufferSubData(
        gl::ARRAY_BUFFER,
 	    0,
 	    size.try_into().unwrap(),
 	    string_buffer_data.as_ptr() as *const c_void);
}

unsafe fn get_uniform_location(program: GLuint, name: &str) -> GLint {
    let name_cstring = CString::new(name).expect("The unexpectable");
    gl::GetUniformLocation(program, name_cstring.as_ptr())
}

fn main() {
    const SCREEN_WIDTH: u32 = 800;
    const SCREEN_HEIGHT: u32 = 600;

    let mut glfw = glfw::init(glfw::FAIL_ON_ERRORS).unwrap();
    let (mut window, events) = glfw
        .create_window(SCREEN_WIDTH, SCREEN_HEIGHT, "Hello this is zozin", glfw::WindowMode::Windowed)
        .expect("Failed to create GLFW window.");

    window.set_key_polling(true);
    window.make_current();

    gl::load_with(|symbol| window.get_proc_address(symbol) as *const _);

    let vertex_shader_source = CString::new(r#"
#version 300 es

precision mediump float;

layout(location=0) in int letter;

uniform vec2 resolution;
uniform vec2 message_position;
uniform float message_scale;

out vec2 uv;

#define FONT_SHEET_WIDTH 128.0
#define FONT_SHEET_HEIGHT 64.0
#define FONT_SHEET_COLS 18
#define FONT_SHEET_ROWS 7
#define FONT_CHAR_WIDTH (FONT_SHEET_WIDTH / float(FONT_SHEET_COLS))
#define FONT_CHAR_HEIGHT (FONT_SHEET_HEIGHT / float(FONT_SHEET_ROWS))

void main() {
    vec2 mesh_position = vec2(
        float(gl_VertexID & 1),
        float((gl_VertexID >> 1) & 1));

    vec2 screen_position =
        mesh_position * vec2(FONT_CHAR_WIDTH, FONT_CHAR_HEIGHT) * message_scale +
        message_position +
        vec2(FONT_CHAR_WIDTH * message_scale * float(gl_InstanceID), 0.0);

    gl_Position = vec4(2.0 * screen_position / resolution, 0.0, 1.0);

    int char_index = letter - 32;
    float char_u = (float(char_index % FONT_SHEET_COLS) + mesh_position.x) * FONT_CHAR_WIDTH / FONT_SHEET_WIDTH;
    float char_v = (float(char_index / FONT_SHEET_COLS) + (1.0 - mesh_position.y)) * FONT_CHAR_HEIGHT / FONT_SHEET_HEIGHT;
    uv = vec2(char_u, char_v);
}
"#).expect("The unexpectable ZULUL");

    let fragment_shader_source = CString::new(r#"
#version 300 es

precision mediump float;

uniform vec2 resolution;
uniform float time;
uniform sampler2D font;

in vec2 uv;
out vec4 color;

void main() {
    color = texture(font, vec2(uv.x, uv.y)) * vec4((sin(time + gl_FragCoord.x / resolution.x) + 1.0) / 2.0, (cos(time + gl_FragCoord.y / resolution.y) + 1.0) / 2.0, 1.0, 1.0);
}
"#).expect("The unexpectable ZULUL");

    let program = unsafe {
        let vertex_shader = compile_shader(&vertex_shader_source, gl::VERTEX_SHADER);
        let fragment_shader = compile_shader(&fragment_shader_source, gl::FRAGMENT_SHADER);
        let program = link_program(&[vertex_shader, fragment_shader]);
        program
    };

    let time_uniform = unsafe { get_uniform_location(program, "time") };
    let resolution_uniform = unsafe { get_uniform_location(program, "resolution") };
    let message_position_uniform = unsafe { get_uniform_location(program, "message_position") };
    let message_scale_uniform = unsafe { get_uniform_location(program, "message_scale") };

    let _font_texture = unsafe {
        load_texture_from_file("./charmap-oldschool_white.png")
    };

    let mut string_buffer_data: [i32; 1024] = [0; 1024];

    let string_buffer_id = unsafe {
        let mut buffer_id = 0;
        gl::GenBuffers(1, &mut buffer_id);
        gl::BindBuffer(gl::ARRAY_BUFFER, buffer_id);
        gl::BufferData(
            gl::ARRAY_BUFFER,
            std::mem::size_of_val(&string_buffer_data).try_into().unwrap(),
            string_buffer_data.as_ptr() as *const c_void,
            gl::DYNAMIC_DRAW
        );
        const CHAR_ATTRIB_INDEX: i32 = 0;
        gl::VertexAttribIPointer(
            CHAR_ATTRIB_INDEX.try_into().unwrap(),
 	        1,
 	        gl::INT,
 	        0,
 	        std::ptr::null());

        gl::EnableVertexAttribArray(CHAR_ATTRIB_INDEX.try_into().unwrap());
        gl::VertexAttribDivisor(CHAR_ATTRIB_INDEX.try_into().unwrap(), 1);
        buffer_id
    };

    let payload = "Hello, World";

    unsafe {
        gl::Uniform2f(resolution_uniform, SCREEN_WIDTH as f32, SCREEN_HEIGHT as f32);
        gl::Uniform2f(message_position_uniform, 0.0, 0.0);
        gl::Uniform1f(message_scale_uniform, 5.0);
        copy_str_to_buffer(&mut string_buffer_data, string_buffer_id, payload);
    }

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

            gl::Uniform1f(time_uniform, glfw.get_time() as f32);
            
            // gl::DrawArrays(gl::TRIANGLE_STRIP, 0, 4);
            gl::DrawArraysInstanced(
                // MAYFAIL: generate mesh based on gl_VertexID and TRIANGLE_STRIP may not work in the instanced setting
                gl::TRIANGLE_STRIP,
 	            0,
 	            4,
 	            payload.len().try_into().unwrap()
            );
        }

        window.swap_buffers();
    }
}
