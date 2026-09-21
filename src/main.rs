mod framebuffer;
mod rayIntersect;
mod sphere;
mod textura;
mod billar;
mod mesa;

use framebuffer::Framebuffer;
use rayIntersect::{Objeto, Ray, cross, escala, normalize, suma};
use billar::crear_bolas;
use mesa::crear_mesa;
use textura::sombrear_puntual;
use raylib::prelude::*;

const WIDTH: i32 = 800;
const HEIGHT: i32 = 600;
const PITCH_MAX: f64 = 1.5;
const LUZ_TECHO: [f64; 3] = [0.0, 4.5, -4.5];
const COLOR_LUZ: [f64; 3] = [1.0, 0.72, 0.30];

/// Direccion hacia la que mira la camara segun su yaw (giro en su eje Y) y pitch (arriba/abajo).
fn direccion_camara(yaw: f64, pitch: f64) -> [f64; 3] {
    [
        yaw.sin() * pitch.cos(),
        pitch.sin(),
        -yaw.cos() * pitch.cos(),
    ]
}

/// Alterna entre modo ventana y pantalla completa al presionar F11.
fn controlar_pantalla_completa(rl: &mut RaylibHandle) {
    if rl.is_key_pressed(KeyboardKey::KEY_F11) {
        rl.toggle_fullscreen();
    }
}

/// Dibuja un contador de FPS legible en la esquina superior izquierda.
fn dibujar_fps<D: RaylibDraw>(d: &mut D, fps: u32) {
    let texto = format!("FPS: {}", fps);
    d.draw_rectangle(8, 8, 112, 34, Color::new(0, 0, 0, 180));
    d.draw_text(&texto, 16, 14, 20, Color::YELLOW);
}

fn main() {
    let (mut rl, thread) = raylib::init()
        .size(WIDTH, HEIGHT)
        .title("Ray Tracing - Bolas de billar")
        .resizable()
        .build();
    rl.set_target_fps(60);

    let bolas = crear_bolas();
    let mesa = crear_mesa();

    // Una sola textura de GPU reutilizada cada frame: subir 480,000 pixeles
    // de una vez es mucho mas rapido que llamar draw_pixel por cada uno.
    let imagen_inicial = Image::gen_image_color(WIDTH, HEIGHT, Color::BLACK);
    let mut pantalla = rl
        .load_texture_from_image(&thread, &imagen_inicial)
        .expect("no se pudo crear la textura de pantalla");

    // Camara libre: se mueve en sus propios ejes (adelante/atras, strafe, arriba/abajo)
    // y gira con yaw (su propio eje Y) y pitch (arriba/abajo).
    let mut camera_pos = [0.0, 1.65, 0.70];
    let mut yaw: f64 = 0.0;
    let mut pitch: f64 = -0.22;
    let velocidad_movimiento = 0.06;
    let velocidad_giro = 0.03;
    let aspect = WIDTH as f64 / HEIGHT as f64;
    let mut fb = Framebuffer::new(WIDTH, HEIGHT, Color::BLACK);

    while !rl.window_should_close() {
        controlar_pantalla_completa(&mut rl);

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

                let mut impacto_cercano: Option<(&dyn Objeto, f64)> = None;

                for bola in &bolas {
                    if let Some(t) = bola.intersect(&ray) {
                        let reemplazar = impacto_cercano
                            .map(|(_, distancia)| t < distancia)
                            .unwrap_or(true);

                        if reemplazar {
                            impacto_cercano = Some((bola as &dyn Objeto, t));
                        }
                    }
                }

                if let Some((pieza, t)) = mesa.intersectar(&ray) {
                    let reemplazar = impacto_cercano
                        .map(|(_, distancia)| t < distancia)
                        .unwrap_or(true);

                    if reemplazar {
                        impacto_cercano = Some((pieza, t));
                    }
                }

                if let Some((objeto, t)) = impacto_cercano {
                    let hit = ray.point_at(t);
                    let normal = objeto.normal(hit);
                    let (u, v) = objeto.uv(hit);
                    let albedo = objeto.textura().albedo(u, v);
                    let shininess = objeto.textura().brillo();
                    let color = sombrear_puntual(
                        ray.direction,
                        hit,
                        normal,
                        LUZ_TECHO,
                        COLOR_LUZ,
                        albedo,
                        shininess,
                    );
                    fb.set_pixel_depth(x, y, color, t);
                }
            }
        }

        let fps = rl.get_fps();
        let ancho_pantalla = rl.get_screen_width();
        let alto_pantalla = rl.get_screen_height();
        let escala_x = ancho_pantalla as f32 / WIDTH as f32;
        let escala_y = alto_pantalla as f32 / HEIGHT as f32;
        let escala_pantalla = escala_x.min(escala_y);
        let desplazamiento_x =
            (ancho_pantalla as f32 - WIDTH as f32 * escala_pantalla) * 0.5;
        let desplazamiento_y =
            (alto_pantalla as f32 - HEIGHT as f32 * escala_pantalla) * 0.5;

        let mut d = rl.begin_drawing(&thread);
        d.clear_background(Color::BLACK);
        pantalla
            .update_texture(fb.as_bytes())
            .expect("no se pudo actualizar la textura de pantalla");
        d.draw_texture_ex(
            &pantalla,
            Vector2::new(desplazamiento_x, desplazamiento_y),
            0.0,
            escala_pantalla,
            Color::WHITE,
        );
        dibujar_fps(&mut d, fps);
    }
}