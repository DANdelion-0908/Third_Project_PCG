use nalgebra_glm::{Vec3, Mat4, look_at, perspective};
use minifb::{Key, Window, WindowOptions};
use std::time::Duration;
use std::f32::consts::PI;

mod framebuffer;
mod triangle;
mod vertex;
mod obj;
mod color;
mod fragment;
mod shaders;
mod camera;

use rodio::{Decoder, OutputStream, source::Source};
use std::fs::File;
use std::io::BufReader;
use std::thread;
use std::sync::mpsc;
use std::sync::Arc;
use std::sync::Mutex;
use framebuffer::Framebuffer;
use vertex::Vertex;
use obj::Obj;
use camera::Camera;
use triangle::triangle;
use shaders::{vertex_shader, fragment_shader};
use fastnoise_lite::{FastNoiseLite, NoiseType};

pub struct Uniforms {
    model_matrix: Mat4,
    view_matrix: Mat4,
    projection_matrix: Mat4,
    viewport_matrix: Mat4,
    time: u32,
    noise: FastNoiseLite
}

pub struct Planet {
    translation: Vec3,
    rotation: Vec3,
    scale: f32,
    vertex_array: Vec<Vertex>,
    shader_selection: u32,
}

fn create_noise() -> FastNoiseLite {
    create_cloud_noise()
}

fn create_cloud_noise() -> FastNoiseLite {
    let mut noise = FastNoiseLite::with_seed(1337);
    noise.set_noise_type(Some(NoiseType::OpenSimplex2));
    noise
}

fn create_model_matrix(translation: Vec3, scale: f32, rotation: Vec3) -> Mat4 {
    let (sin_x, cos_x) = rotation.x.sin_cos();
    let (sin_y, cos_y) = rotation.y.sin_cos();
    let (sin_z, cos_z) = rotation.z.sin_cos();

    let rotation_matrix_x = Mat4::new(
        1.0,  0.0,    0.0,   0.0,
        0.0,  cos_x, -sin_x, 0.0,
        0.0,  sin_x,  cos_x, 0.0,
        0.0,  0.0,    0.0,   1.0,
    );

    let rotation_matrix_y = Mat4::new(
        cos_y,  0.0,  sin_y, 0.0,
        0.0,    1.0,  0.0,   0.0,
        -sin_y, 0.0,  cos_y, 0.0,
        0.0,    0.0,  0.0,   1.0,
    );

    let rotation_matrix_z = Mat4::new(
        cos_z, -sin_z, 0.0, 0.0,
        sin_z,  cos_z, 0.0, 0.0,
        0.0,    0.0,  1.0, 0.0,
        0.0,    0.0,  0.0, 1.0,
    );

    let rotation_matrix = rotation_matrix_z * rotation_matrix_y * rotation_matrix_x;

    let transform_matrix = Mat4::new(
        scale, 0.0,   0.0,   translation.x,
        0.0,   scale, 0.0,   translation.y,
        0.0,   0.0,   scale, translation.z,
        0.0,   0.0,   0.0,   1.0,
    );

    transform_matrix * rotation_matrix
}


fn create_view_matrix(eye: Vec3, center: Vec3, up: Vec3) -> Mat4 {
    look_at(&eye, &center, &up)
}

fn create_perspective_matrix(window_width: f32, window_height: f32) -> Mat4 {
    let fov = 45.0 * PI / 180.0;
    let aspect_ratio = window_width / window_height;
    let near = 0.1;
    let far = 1000.0;

    perspective(fov, aspect_ratio, near, far)
}

fn create_viewport_matrix(width: f32, height: f32) -> Mat4 {
    Mat4::new(
        width / 2.0, 0.0, 0.0, width / 2.0,
        0.0, -height / 2.0, 0.0, height / 2.0,
        0.0, 0.0, 1.0, 0.0,
        0.0, 0.0, 0.0, 1.0
    )
}

fn play_music(file_path: &str, stop_signal: Arc<Mutex<bool>>) {
    // Crea un nuevo stream de salida
    let (_stream, stream_handle) = OutputStream::try_default().unwrap();

    // Abre el archivo de audio
    let file = File::open(file_path).unwrap();
    let source = Decoder::new(BufReader::new(file)).unwrap();

    // Reproduce la música
    stream_handle.play_raw(source.convert_samples()).unwrap();

    // Mantén el programa corriendo mientras se reproduce la música
    while !*stop_signal.lock().unwrap() {
        thread::sleep(std::time::Duration::from_millis(100));
    }
}

fn render(framebuffer: &mut Framebuffer, uniforms: &Uniforms, vertex_array: &[Vertex], shader_selection: u32) {
    // Vertex Shader
    let mut transformed_vertices = Vec::with_capacity(vertex_array.len());
    for vertex in vertex_array {
        let transformed = vertex_shader(vertex, uniforms);
        transformed_vertices.push(transformed);
    }

    // Primitive Assembly
    let mut triangles = Vec::new();
    for i in (0..transformed_vertices.len()).step_by(3) {
        if i + 2 < transformed_vertices.len() {
            triangles.push([
                transformed_vertices[i].clone(),
                transformed_vertices[i + 1].clone(),
                transformed_vertices[i + 2].clone(),
            ]);
        }
    }

    // Rasterization
    let mut fragments = Vec::new();
    for tri in &triangles {
        fragments.extend(triangle(&tri[0], &tri[1], &tri[2], shader_selection));
    }

    // Fragment Processing
    for fragment in fragments {
        let x = fragment.position.x as usize;
        let y = fragment.position.y as usize;
        let mut shaded_color = fragment_shader(&fragment, &uniforms, "lava");

        if x < framebuffer.width && y < framebuffer.height {
            if shader_selection == 0 {
                shaded_color = fragment_shader(&fragment, &uniforms, "lava");
            } else if shader_selection == 1 {
                shaded_color = fragment_shader(&fragment, &uniforms, "ice");
            } else if shader_selection == 2 {
                shaded_color = fragment_shader(&fragment, &uniforms, "cloud");
            } else if shader_selection == 3 {
                shaded_color = fragment_shader(&fragment, &uniforms, "jupiter");
            } else if shader_selection == 4{
                shaded_color = fragment_shader(&fragment, &uniforms, "ring");
            } else if shader_selection == 5{
                shaded_color = fragment_shader(&fragment, &uniforms, "metal");
            }
            let color = shaded_color.to_hex();
            framebuffer.set_current_color(color);
            framebuffer.point(x, y, fragment.depth);
        }
    }
}

fn main() {
    let file_path = "assets/music/Good Egg Galaxy  Super Mario Galaxy.mp3";
    let stop_signal = Arc::new(Mutex::new(false));
    let stop_signal_clone = Arc::clone(&stop_signal);

    let music_thread = thread::spawn(move || {
        play_music(file_path, stop_signal_clone);
    });

    let window_width = 800;
    let window_height = 600;
    let framebuffer_width = 800;
    let framebuffer_height = 600;
    let frame_delay = Duration::from_millis(16);
    let mut shader_selection = 0;

    // Configuración de planetas
    let mut planets = vec![
        Planet {
            translation: Vec3::new(0.0, 0.0, 0.0), // El Sol en el centro
            rotation: Vec3::new(0.0, 0.0, 0.0),
            scale: 1.5, // Tamaño mayor para el Sol
            vertex_array: Obj::load("assets/models/sphere.obj")
                .expect("Failed to load sphere.obj")
                .get_vertex_array(),
            shader_selection: 0, // Shader para el Sol
        },
        Planet {
            translation: Vec3::new(3.0, 0.0, 0.0), // Posición inicial del planeta
            rotation: Vec3::new(0.0, 0.0, 0.0),
            scale: 0.5, // Tamaño del planeta
            vertex_array: Obj::load("assets/models/sphere.obj")
                .expect("Failed to load planet.obj")
                .get_vertex_array(),
            shader_selection: 1, // Shader para el planeta
        },
        Planet {
            translation: Vec3::new(4.0, 0.0, 0.0), // Posición inicial del planeta
            rotation: Vec3::new(0.0, 0.0, 0.0),
            scale: 0.5, // Tamaño del planeta
            vertex_array: Obj::load("assets/models/sphere.obj")
                .expect("Failed to load planet.obj")
                .get_vertex_array(),
            shader_selection: 2, // Shader para el planeta
        },
        Planet {
            translation: Vec3::new(6.0, 0.0, 0.0), // Posición inicial del planeta
            rotation: Vec3::new(0.0, 0.0, 0.0),
            scale: 0.5, // Tamaño del planeta
            vertex_array: Obj::load("assets/models/sphere.obj")
                .expect("Failed to load planet.obj")
                .get_vertex_array(),
            shader_selection: 3, // Shader para el planeta
        },
        Planet {
            translation: Vec3::new(8.0, 0.0, 0.0), // Posición inicial del planeta
            rotation: Vec3::new(0.0, 0.0, 0.0),
            scale: 0.5, // Tamaño del planeta
            vertex_array: Obj::load("assets/models/sphere.obj")
                .expect("Failed to load planet.obj")
                .get_vertex_array(),
            shader_selection: 4, // Shader para el planeta
        },
        Planet {
            translation: Vec3::new(10.0, 0.0, 0.0), // Posición inicial del planeta
            rotation: Vec3::new(0.0, 0.0, 0.0),
            scale: 0.5, // Tamaño del planeta
            vertex_array: Obj::load("assets/models/sphere.obj")
                .expect("Failed to load planet.obj")
                .get_vertex_array(),
            shader_selection: 5, // Shader para el planeta
        },
    ];

    let mut framebuffer = Framebuffer::new(framebuffer_width, framebuffer_height);
    let mut window = Window::new(
        "Simulador del sistema planetario",
        window_width,
        window_height,
        WindowOptions::default(),
    )
    .unwrap();

    window.set_position(500, 500);
    window.update();

    framebuffer.set_background_color(0x000);

    // Parámetros de la cámara
    let mut camera = Camera::new(
        Vec3::new(0.0, 0.0, 30.0), // Alejamos la cámara para ver todo el sistema
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
    );

    let mut time = 0;

    while window.is_open() {
        if window.is_key_down(Key::Escape) {
            break;
        }

        time += 1;

        shader_selection = handle_input(&window, &mut camera, shader_selection);

        framebuffer.clear();

        // Matrices de vista y proyección
        let view_matrix = create_view_matrix(camera.eye, camera.center, camera.up);
        let projection_matrix = create_perspective_matrix(window_width as f32, window_height as f32);
        let viewport_matrix = create_viewport_matrix(
            framebuffer_width as f32,
            framebuffer_height as f32,
        );

        for (index, planet) in planets.iter_mut().enumerate() {
            if index == 0 {
                // El Sol no se mueve
                planet.rotation.y += 0.02; // Rotación del Sol
            } else {
                // Los planetas orbitan alrededor del Sol
                let angle = time as f32 * 0.01 * (index as f32); // Ángulo para la órbita
                let distance = 3.0 + (index as f32) * 2.0; // Distancia desde el Sol
                planet.translation.x = distance * angle.cos();
                planet.translation.z = distance * angle.sin();
                planet.rotation.y += 0.02; // Rotación del planeta
            }

            let model_matrix = create_model_matrix(
                planet.translation,
                planet.scale,
                planet.rotation,
            );

            let uniforms = Uniforms {
                model_matrix,
                view_matrix,
                projection_matrix,
                viewport_matrix,
                time,
                noise: create_noise(),
            };

            render(
                &mut framebuffer,
                &uniforms,
                &planet.vertex_array,
                planet.shader_selection,
            );
        }

        window
            .update_with_buffer(&framebuffer.buffer, framebuffer_width, framebuffer_height)
            .unwrap();

        std::thread::sleep(frame_delay);
    }

    *stop_signal.lock().unwrap() = true;
    music_thread.join().unwrap();
}

fn handle_input(window: &Window, camera: &mut Camera, mut shader_selection: u32) -> u32 {
    let movement_speed = 1.0;
    let rotation_speed = PI/50.0;
    let zoom_speed = 0.1;
   
    //  camera orbit controls
    if window.is_key_down(Key::Left) {
      camera.orbit(rotation_speed, 0.0);
    }
    if window.is_key_down(Key::Right) {
      camera.orbit(-rotation_speed, 0.0);
    }
    if window.is_key_down(Key::W) {
      camera.orbit(0.0, -rotation_speed);
    }
    if window.is_key_down(Key::S) {
      camera.orbit(0.0, rotation_speed);
    }

    // Camera movement controls
    let mut movement = Vec3::new(0.0, 0.0, 0.0);
    if window.is_key_down(Key::A) {
      movement.x -= movement_speed;
    }
    if window.is_key_down(Key::D) {
      movement.x += movement_speed;
    }
    if window.is_key_down(Key::Q) {
      movement.y += movement_speed;
    }
    if window.is_key_down(Key::E) {
      movement.y -= movement_speed;
    }
    if movement.magnitude() > 0.0 {
      camera.move_center(movement);
    }

    // Camera zoom controls
    if window.is_key_down(Key::Up) {
      camera.zoom(zoom_speed);
    }
    if window.is_key_down(Key::Down) {
      camera.zoom(-zoom_speed);
    }

    // Shader selection controls
    if window.is_key_down(Key::NumPad0) {
        shader_selection = 0;
    }

    if window.is_key_down(Key::NumPad1) {
        shader_selection = 1;
    }

    if window.is_key_down(Key::NumPad2) {
        shader_selection = 2;
    }

    if window.is_key_down(Key::NumPad3) {
        shader_selection = 3;
    }

    return shader_selection;
}
