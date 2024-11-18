
use nalgebra_glm::{mat4_to_mat3, Mat3, Vec2, Vec3, Vec4};
use crate::vertex::Vertex;
use crate::Uniforms;
use crate::fragment::Fragment;
use crate::color::Color;

pub fn vertex_shader(vertex: &Vertex, uniforms: &Uniforms) -> Vertex {
    let position = Vec4::new(
        vertex.position.x,
        vertex.position.y,
        vertex.position.z,
        1.0
    );

    let transformed = uniforms.projection_matrix * uniforms.view_matrix * uniforms.model_matrix * position;

    let w = transformed.w;
    let transformed_position = Vec4::new(
        transformed.x / w,
        transformed.y / w,
        transformed.z / w,
        1.0
    );

    let screen_position = uniforms.viewport_matrix * transformed_position;

    let model_mat3 = mat4_to_mat3(&uniforms.model_matrix);
    let normal_matrix = model_mat3.transpose().try_inverse().unwrap_or(Mat3::identity());

    let transformed_normal = normal_matrix * vertex.normal;

    Vertex {
        position: vertex.position,
        normal: vertex.normal,
        tex_coords: vertex.tex_coords,
        color: vertex.color,
        transformed_position: Vec3::new(screen_position.x, screen_position.y, screen_position.z),
        transformed_normal: transformed_normal
    }
}

pub fn fragment_shader(fragment: &Fragment, uniforms: &Uniforms, shader_type: &str) -> Color {
  match shader_type {
      "cloud" => cloud_shader(fragment, uniforms),
      "lava" => lava_shader(fragment, uniforms),
      "ice" => ice_shader(fragment, uniforms),
      "jupiter" => jupiter_shader(fragment, uniforms),
      "ring" => ring_shader(fragment, uniforms),
      "metal" => metal_shader(fragment, uniforms),
      _ => combined_shader(fragment, uniforms), // Default shader
  }
}

fn static_pattern_shader(fragment: &Fragment) -> Color {
    let x = fragment.vertex_position.x;
    let y = fragment.vertex_position.y;
  
    let pattern = ((x * 10.0).sin() * (y * 10.0).sin()).abs();
  
    let r = (pattern * 255.0) as u8;
    let g = ((1.0 - pattern) * 255.0) as u8;
    let b = 128;
  
    Color::new(r, g, b)
}

fn lava_shader(fragment: &Fragment, uniforms: &Uniforms) -> Color {
  // Base colors for the lava effect
  let bright_color = Color::new(255, 240, 0); // Bright orange (lava-like)
  let dark_color = Color::new(130, 20, 0);   // Darker red-orange  

  // Get fragment position
  let position = Vec3::new(
    fragment.vertex_position.x,
    fragment.vertex_position.y,
    fragment.depth
  );

  // Base frequency and amplitude for the pulsating effect
  let base_frequency = 0.2;
  let pulsate_amplitude = 0.5;
  let t = uniforms.time as f32 * 0.01;

  // Pulsate on the z-axis to change spot size
  let pulsate = (t * base_frequency).sin() * pulsate_amplitude;

  // Apply noise to coordinates with subtle pulsating on z-axis
  let zoom = 1000.0; // Constant zoom factor
  let noise_value1 = uniforms.noise.get_noise_3d(
    position.x * zoom,
    position.y * zoom,
    (position.z + pulsate) * zoom
  );
  let noise_value2 = uniforms.noise.get_noise_3d(
    (position.x + 1000.0) * zoom,
    (position.y + 1000.0) * zoom,
    (position.z + 1000.0 + pulsate) * zoom
  );
  let noise_value = (noise_value1 + noise_value2) * 0.5;  // Averaging noise for smoother transitions

  // Use lerp for color blending based on noise value
  let color = dark_color.lerp(&bright_color, noise_value);

  color * fragment.intensity
}

fn ice_shader(fragment: &Fragment, uniforms: &Uniforms) -> Color {
  let ripple_pattern = (fragment.vertex_position.x * 8.0 + uniforms.time as f32 * 0.1).sin().abs();
  let intensity = (ripple_pattern * 255.0) as u8;
  Color::new(0, intensity, 255) * fragment.intensity // Azul agua
}

fn cloud_shader(fragment: &Fragment, uniforms: &Uniforms) -> Color {
  let zoom = 100.0;  // Escala del mapa de ruido
  let ox = 100.0; // Offset en el eje x
  let oy = 100.0; // Offset en el eje y

  // Posición del fragmento en el espacio de coordenadas
  let x = fragment.vertex_position.x;
  let y = fragment.vertex_position.y;

  // Tiempos diferentes para las nubes y el terreno
  let cloud_time = uniforms.time as f32 * 0.5;  // Las nubes se mueven a un ritmo
  let land_time = uniforms.time as f32 * 0.2;   // El terreno se mueve a otro ritmo

  // Obtener el valor de ruido para las nubes y el terreno con sus respectivos tiempos
  let cloud_noise = uniforms.noise.get_noise_2d(x * zoom + ox + cloud_time, y * zoom + oy);
  let land_noise = uniforms.noise.get_noise_2d(x * zoom + ox + land_time, y * zoom + oy);

  // Umbrales de nubes y tierra
  let cloud_threshold = 0.5;
  let land_threshold = 0.1;

  // Colores para nubes, cielo y tierra
  let cloud_color = Color::new(255, 255, 255); // Blanco para nubes
  let sky_color = Color::new(30, 97, 145);     // Azul para el cielo
  let land_color = Color::new(0, 100, 0);      // Verde para tierra

  // Decidir el color final basado en los umbrales
  let final_color = if cloud_noise > cloud_threshold {
      cloud_color  // Color de nubes
  } else if land_noise > land_threshold {
      land_color   // Color de tierra
  } else {
      sky_color    // Color del cielo
  };

  final_color * fragment.intensity
}

fn metal_shader(fragment: &Fragment, uniforms: &Uniforms) -> Color {
  let position = fragment.vertex_position;
  let normal = fragment.normal.normalize();

  // Luz direccional
  let light_dir = Vec3::new(0.5, 0.5, 1.0).normalize();
  let dot_product = normal.dot(&light_dir).max(0.0);

  // Colores base
  let base_color = Color::new(100, 100, 120); // Gris metálico
  let highlight_color = Color::new(220, 220, 255); // Azul brillante

  // Mezclar en función del ángulo con la luz
  base_color.lerp(&highlight_color, dot_product) * fragment.intensity
}


fn jupiter_shader(fragment: &Fragment, uniforms: &Uniforms) -> Color {
  let zoom = 100.0;  // to move our values 
  let x = fragment.vertex_position.x;
  let y = fragment.vertex_position.y;

  let band_noise = uniforms.noise.get_noise_2d(y * zoom, 0.0);// Desplazamiento para el movimiento de bandas

  // Definir colores para las diferentes bandas de gas
  let dark_brown = Color::new(139, 69, 19);
  let light_brown = Color::new(205, 133, 63);
  let orange = Color::new(255, 165, 0);
  let beige = Color::new(245, 222, 179);

  // Crear bandas con variación de color usando `band_noise`
  let band_color = if band_noise > 0.6 {
      dark_brown
  } else if band_noise > 0.3 {
      beige
  } else if band_noise > 0.0 {
      orange
  } else {
      light_brown
  };

  // Agregar una "mancha" similar a la Gran Mancha Roja
  let storm_position = Vec2::new(0.3, -0.3); // Posición en el planeta
  let storm_radius = 0.15; // Tamaño de la tormenta
  let distance_to_storm = ((x - storm_position.x).powi(2) + (y - storm_position.y).powi(2)).sqrt();

  let storm_color = Color::new(255, 69, 0); // Rojo anaranjado brillante para la tormenta
  let final_color = if distance_to_storm < storm_radius {
      storm_color // La región de la tormenta
  } else {
      band_color // Colores de bandas para el resto
  };

  final_color * fragment.intensity
}

fn ring_shader(fragment: &Fragment, uniforms: &Uniforms) -> Color {
  let x = fragment.vertex_position.x;
  let y = fragment.vertex_position.y;
  let z = fragment.vertex_position.z;

  // Coordenadas polares
  let distance = (x.powi(2) + y.powi(2)).sqrt();
  let angle = y.atan2(x);

  // Parámetros del anillo
  let ring_width = 0.02; // Ancho del anillo
  let ring_spacing = 0.08; // Espaciado entre anillos

  // Crear patrón de anillos
  let ring_pattern = ((distance % ring_spacing) / ring_width).abs();
  let ring_intensity = if ring_pattern < 1.0 { 1.0 - ring_pattern } else { 0.0 };

  // Definir colores del anillo y del planeta
  let ring_color = Color::new(200, 200, 200); // Gris para los anillos
  let planet_color = Color::new(100, 50, 200); // Morado para el planeta

  // Interpolar entre el color del planeta y el de los anillos
  ring_color.lerp(&planet_color, 1.0 - ring_intensity) * fragment.intensity
}


fn moving_circles_shader(fragment: &Fragment, uniforms: &Uniforms) -> Color {
    let x = fragment.vertex_position.x;
    let y = fragment.vertex_position.y;
  
    let time = uniforms.time as f32 * 0.05;
    let circle1_x = (time.sin() * 0.4 + 0.5) % 1.0;
    let circle2_x = (time.cos() * 0.4 + 0.5) % 1.0;
  
    let dist1 = ((x - circle1_x).powi(2) + (y - 0.3).powi(2)).sqrt();
    let dist2 = ((x - circle2_x).powi(2) + (y - 0.7).powi(2)).sqrt();
  
    let circle_size = 0.1;
    let circle1 = if dist1 < circle_size { 1.0f32 } else { 0.0f32 };
    let circle2 = if dist2 < circle_size { 1.0f32 } else { 0.0f32 };
  
    let circle_intensity = (circle1 + circle2).min(1.0f32);
  
    Color::new(
      (circle_intensity * 255.0) as u8,
      (circle_intensity * 255.0) as u8,
      (circle_intensity * 255.0) as u8
    )
}

pub fn combined_shader(fragment: &Fragment, uniforms: &Uniforms) -> Color {
    let base_color = static_pattern_shader(fragment);
    let circle_color = moving_circles_shader(fragment, uniforms);
  
    // Combine shaders: use circle color if it's not black, otherwise use base color
    if !circle_color.is_black() {
      circle_color * fragment.intensity
    } else {
      base_color * fragment.intensity
    }
}