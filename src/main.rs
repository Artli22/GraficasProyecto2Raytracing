mod framebuffer;
mod rayIntersect;
mod sphere;
mod cubo;
mod textura;

use framebuffer::Framebuffer;
use rayIntersect::{Objeto, Ray, cross, escala, normalize, suma};
use sphere::Sphere;
use cubo::Cubo;
use textura::{Canica, sombrear};
use raylib::prelude::*;

const WIDTH: i32 = 800;
const HEIGHT: i32 = 600;
const PITCH_MAX: f64 = 1.5;

/// Direccion hacia la que mira la camara segun su yaw (giro en su eje Y) y pitch (arriba/abajo).
fn direccion_camara(yaw: f64, pitch: f64) -> [f64; 3] {
    [
        yaw.sin() * pitch.cos(),
        pitch.sin(),
        -yaw.cos() * pitch.cos(),
    ]
}

fn main() {
    let (mut rl, thread) = raylib::init()
        .size(WIDTH, HEIGHT)
        .title("Ray Tracing - Sphere")
        .build();

    let esfera = Sphere::new(
        [0.0, 0.0, -3.0],
        1.0,
        Box::new(Canica::new([1.0, 0.5, 0.0], 50.0)),
    );
    let cubo = Cubo::new(
        [0.5, 0.3, -1.5],
        0.4,
        Box::new(Canica::new([1.0, 0.0, 0.0], 80.0)),
    );
    let objetos: Vec<&dyn Objeto> = vec![&cubo];

    // Una sola textura de GPU reutilizada cada frame: subir 480,000 pixeles
    // de una vez es mucho mas rapido que llamar draw_pixel por cada uno.
    let imagen_inicial = Image::gen_image_color(WIDTH, HEIGHT, Color::BLACK);
    let mut pantalla = rl
        .load_texture_from_image(&thread, &imagen_inicial)
        .expect("no se pudo crear la textura de pantalla");

    // Camara libre: se mueve en sus propios ejes (adelante/atras, strafe, arriba/abajo)
    // y gira con yaw (su propio eje Y) y pitch (arriba/abajo).
    let mut camera_pos = [0.2, 0.1, 0.5];
    let mut yaw: f64 = 0.0;
    let mut pitch: f64 = 0.0;
    let velocidad_movimiento = 0.06;
    let velocidad_giro = 0.03;
    let aspect = WIDTH as f64 / HEIGHT as f64;
    let light_dir = normalize([0.5, 0.5, 1.0]);
    let mut fb = Framebuffer::new(WIDTH, HEIGHT, Color::BLACK);

    while !rl.window_should_close() {
        if rl.is_key_down(KeyboardKey::KEY_LEFT) {
            yaw -= velocidad_giro;
        }
        if rl.is_key_down(KeyboardKey::KEY_RIGHT) {
            yaw += velocidad_giro;
        }
        if rl.is_key_down(KeyboardKey::KEY_UP) {
            pitch = (pitch + velocidad_giro).min(PITCH_MAX);
        }
        if rl.is_key_down(KeyboardKey::KEY_DOWN) {
            pitch = (pitch - velocidad_giro).max(-PITCH_MAX);
        }

        let forward = normalize(direccion_camara(yaw, pitch));
        let right = normalize(cross(forward, [0.0, 1.0, 0.0]));
        let up = cross(right, forward);

        if rl.is_key_down(KeyboardKey::KEY_W) {
            camera_pos = suma(camera_pos, escala(forward, velocidad_movimiento));
        }
        if rl.is_key_down(KeyboardKey::KEY_S) {
            camera_pos = suma(camera_pos, escala(forward, -velocidad_movimiento));
        }
        if rl.is_key_down(KeyboardKey::KEY_A) {
            camera_pos = suma(camera_pos, escala(right, -velocidad_movimiento));
        }
        if rl.is_key_down(KeyboardKey::KEY_D) {
            camera_pos = suma(camera_pos, escala(right, velocidad_movimiento));
        }
        if rl.is_key_down(KeyboardKey::KEY_SPACE) {
            camera_pos[1] += velocidad_movimiento;
        }
        if rl.is_key_down(KeyboardKey::KEY_LEFT_SHIFT) {
            camera_pos[1] -= velocidad_movimiento;
        }

        fb.clear(Color::BLACK);

        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                let u = (x as f64 + 0.5) / WIDTH as f64 * 2.0 - 1.0;
                let v = 1.0 - (y as f64 + 0.5) / HEIGHT as f64 * 2.0;
                let dir = [
                    forward[0] + right[0] * u * aspect + up[0] * v,
                    forward[1] + right[1] * u * aspect + up[1] * v,
                    forward[2] + right[2] * u * aspect + up[2] * v,
                ];
                let ray = Ray::new(camera_pos, dir);

                for objeto in &objetos {
                    if let Some(t) = objeto.intersect(&ray) {
                        let hit = ray.point_at(t);
                        let normal = objeto.normal(hit);
                        let (u, v) = objeto.uv(hit);
                        let albedo = objeto.textura().albedo(u, v);
                        let shininess = objeto.textura().brillo();
                        let color = sombrear(ray.direction, normal, light_dir, albedo, shininess);
                        fb.set_pixel_depth(x, y, color, t);
                    }
                }
            }
        }

        let mut d = rl.begin_drawing(&thread);
        d.clear_background(Color::BLACK);
        pantalla
            .update_texture(fb.as_bytes())
            .expect("no se pudo actualizar la textura de pantalla");
        d.draw_texture(&pantalla, 0, 0, Color::WHITE);
    }
}

