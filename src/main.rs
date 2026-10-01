mod billar;
mod cuarto;
mod framebuffer;
mod lampara;
mod mesa;
mod paralelo;
mod patas;
mod rayIntersect;
mod render;
mod sombras;
mod sphere;
mod taco;
mod textura;
mod vasos;

use billar::crear_bolas;
use framebuffer::Framebuffer;
use mesa::crear_mesa;
use paralelo::{Camara, RenderParalelo};
use rayIntersect::{Objeto, Ray, cross, escala, normalize, suma};
use raylib::prelude::*;

const WIDTH: i32 = 800;
const HEIGHT: i32 = 600;
const PITCH_MAX: f64 = 1.5;

/// Direccion hacia la que mira la camara segun su yaw y pitch 
fn direccion_camara(yaw: f64, pitch: f64) -> [f64; 3] {
    [
        yaw.sin() * pitch.cos(),
        pitch.sin(),
        -yaw.cos() * pitch.cos(),
    ]
}

/// Alterna entre modo ventana y pantalla completa 
fn controlar_pantalla_completa(rl: &mut RaylibHandle) {
    if rl.is_key_pressed(KeyboardKey::KEY_F11) {
        rl.toggle_fullscreen();
    }
}

/// Dibuja un contador de FPS legible 
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
    let lampara = lampara::crear_lampara();
    let habitacion = cuarto::crear_habitacion();

    // Una sola textura de GPU reutilizada cada frame
    let imagen_inicial = Image::gen_image_color(WIDTH, HEIGHT, Color::BLACK);
    let mut pantalla = rl
        .load_texture_from_image(&thread, &imagen_inicial)
        .expect("no se pudo crear la textura de pantalla");

    // Camara libre
    let mut camera_pos = [0.0, 1.65 + mesa::ELEVACION_MESA, 0.70];
    let mut yaw: f64 = 0.0;
    let mut pitch: f64 = -0.22;
    let velocidad_movimiento = 0.06;
    let velocidad_giro = 0.03;
    let aspect = WIDTH as f64 / HEIGHT as f64;
    let mut fb = Framebuffer::new(WIDTH, HEIGHT, Color::BLACK);

    // Sombras no dinamicas 
    let sombras =
        sombras::MapaSombras::construir(lampara::LUZ_POSICION, sombras::RESOLUCION, &|r| {
            let mut cercano = mesa
                .intersectar_opacos(r)
                .map(|(_, t)| t)
                .unwrap_or(f64::INFINITY);
            for b in &bolas {
                if let Some(t) = b.intersect(r) {
                    cercano = cercano.min(t);
                }
            }
            if let Some((_, t)) = lampara.intersectar(r) {
                cercano = cercano.min(t);
            }
            cercano.is_finite().then_some(cercano)
        });

    // La escena se comparte por referencias inmutables mientras exista el pool.
    let calcular_pixel = |x: usize, y: usize, camara: &Camara| -> Color {
        let u = (x as f64 + 0.5) / WIDTH as f64 * 2.0 - 1.0;
        let v = 1.0 - (y as f64 + 0.5) / HEIGHT as f64 * 2.0;
        let dir = [
            camara.forward[0] + camara.right[0] * u * aspect + camara.up[0] * v,
            camara.forward[1] + camara.right[1] * u * aspect + camara.up[1] * v,
            camara.forward[2] + camara.right[2] * u * aspect + camara.up[2] * v,
        ];
        let ray = Ray::new(camara.posicion, dir);

        render::trazar_precalculado(
            &ray,
            &|ray: &Ray| {
                let mut impacto_cercano: Option<(&dyn Objeto, f64)> = None;

                for bola in &bolas {
                    if let Some(t) = bola.intersect(ray) {
                        let reemplazar = impacto_cercano
                            .map(|(_, distancia)| t < distancia)
                            .unwrap_or(true);

                        if reemplazar {
                            impacto_cercano = Some((bola as &dyn Objeto, t));
                        }
                    }
                }

                if let Some((pieza, t)) = mesa.intersectar(ray) {
                    let reemplazar = impacto_cercano
                        .map(|(_, distancia)| t < distancia)
                        .unwrap_or(true);

                    if reemplazar {
                        impacto_cercano = Some((pieza, t));
                    }
                }

                if let Some((pieza, t)) = lampara.intersectar(ray) {
                    if impacto_cercano.map(|(_, d)| t < d).unwrap_or(true) {
                        impacto_cercano = Some((pieza, t));
                    }
                }
                let mut es_habitacion = false;
                if let Some((panel, t)) = habitacion.intersectar(ray) {
                    if impacto_cercano.map(|(_, d)| t < d).unwrap_or(true) {
                        impacto_cercano = Some((panel, t));
                        es_habitacion = true;
                    }
                }
                impacto_cercano.map(|(objeto, t)| (objeto, t, es_habitacion))
            },
            &|p, n, cuarto| sombras.visibilidad(p, n, cuarto),
        )
    };
    let trabajadores = std::env::var("RENDER_THREADS")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .filter(|&n| n > 0)
        .unwrap_or_else(paralelo::trabajadores_disponibles);
    let filas_bloque = std::env::var("RENDER_BLOCK_ROWS")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .filter(|&n| n > 0)
        .unwrap_or(8);
    std::thread::scope(|scope| {
        let mut pool = RenderParalelo::new(
            scope,
            WIDTH as usize,
            HEIGHT as usize,
            trabajadores,
            filas_bloque,
            &calcular_pixel,
        );
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

            // Aplicar despues de TODOS los controles y antes de trazar los rayos.
            cuarto::limitar_camara(&mut camera_pos);

            pool.renderizar(
                Camara {
                    posicion: camera_pos,
                    forward,
                    right,
                    up,
                },
                fb.pixels_mut(),
            )
            .expect("fallo del render paralelo");
            let fps = rl.get_fps();
            let ancho_pantalla = rl.get_screen_width();
            let alto_pantalla = rl.get_screen_height();
            let escala_x = ancho_pantalla as f32 / WIDTH as f32;
            let escala_y = alto_pantalla as f32 / HEIGHT as f32;
            let escala_pantalla = escala_x.min(escala_y);
            let desplazamiento_x = (ancho_pantalla as f32 - WIDTH as f32 * escala_pantalla) * 0.5;
            let desplazamiento_y = (alto_pantalla as f32 - HEIGHT as f32 * escala_pantalla) * 0.5;

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
    });
}
