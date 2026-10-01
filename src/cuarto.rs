use crate::rayIntersect::{Objeto, Ray};
use crate::textura::{Albedo, ColorSolido, Textura, TexturaMadera};

const EPS: f64 = 1e-7;

// Edita estos limites para cambiar el tamano: geometria y camara los comparten.
pub const X_MIN: f64 = -5.0;
pub const X_MAX: f64 = 5.0;
pub const Y_MIN: f64 = 0.0; // Piso.
pub const Y_MAX: f64 = 5.2; // Techo.
pub const Z_MIN: f64 = -11.0;
pub const Z_MAX: f64 = 3.0;
pub const MARGEN_CAMARA: f64 = 0.20;

/// Mantiene la camara dentro, dejando una separacion de cada superficie.
/// Limitar cada eje permite deslizarse junto a paredes y detenerse en esquinas.
pub fn limitar_camara(posicion: &mut [f64; 3]) {
    let minimo = [X_MIN, Y_MIN, Z_MIN];
    let maximo = [X_MAX, Y_MAX, Z_MAX];
    for eje in 0..3 {
        posicion[eje] =
            posicion[eje].clamp(minimo[eje] + MARGEN_CAMARA, maximo[eje] - MARGEN_CAMARA);
    }
}

/// Rectangulo finito alineado con un plano coordenado, sin grosor.
pub struct Panel {
    eje: usize,
    posicion: f64,
    ejes_uv: [usize; 2],
    minimo: [f64; 2],
    maximo: [f64; 2],
    signo_normal: f64,
    material: Box<dyn Textura>,
}

impl Panel {
    fn new(
        eje: usize,
        posicion: f64,
        minimo: [f64; 2],
        maximo: [f64; 2],
        signo_normal: f64,
        material: Box<dyn Textura>,
    ) -> Self {
        let ejes_uv = match eje {
            0 => [1, 2],
            1 => [0, 2],
            2 => [0, 1],
            _ => panic!("eje invalido"),
        };
        Self {
            eje,
            posicion,
            ejes_uv,
            minimo,
            maximo,
            signo_normal,
            material,
        }
    }
}

impl Objeto for Panel {
    fn intersect(&self, ray: &Ray) -> Option<f64> {
        let denominador = ray.direction[self.eje];
        if denominador.abs() < EPS {
            return None;
        }
        let t = (self.posicion - ray.origin[self.eje]) / denominador;
        if t <= EPS {
            return None;
        }
        let p = ray.point_at(t);
        for i in 0..2 {
            if p[self.ejes_uv[i]] < self.minimo[i] - EPS
                || p[self.ejes_uv[i]] > self.maximo[i] + EPS
            {
                return None;
            }
        }
        Some(t)
    }

    fn normal(&self, _: [f64; 3]) -> [f64; 3] {
        let mut normal = [0.0; 3];
        normal[self.eje] = self.signo_normal;
        normal
    }

    fn uv(&self, p: [f64; 3]) -> (f64, f64) {
        // Coordenadas en unidades de mundo para mantener el tamano de las tablas.
        (
            p[self.ejes_uv[0]] - self.minimo[0],
            p[self.ejes_uv[1]] - self.minimo[1],
        )
    }

    fn textura(&self) -> &dyn Textura {
        self.material.as_ref()
    }
}

struct PisoTablas {
    madera: TexturaMadera,
}

impl Textura for PisoTablas {
    fn albedo(&self, u: f64, v: f64) -> Albedo {
        let ancho = 0.65;
        let largo = 2.6;
        let fila = (u / ancho).floor();
        let desplazamiento = fila.rem_euclid(2.0) * largo * 0.5;
        let x = u.rem_euclid(ancho);
        let z = (v + desplazamiento).rem_euclid(largo);
        if x < 0.008 || z < 0.008 {
            return [0.13, 0.075, 0.04];
        }
        let pieza = ((v + desplazamiento) / largo).floor();
        let tono = 0.92 + (fila * 17.0 + pieza * 7.0).rem_euclid(9.0) * 0.02;
        // Vetas largas orientadas con las tablas, usando la textura existente.
        let base = self.madera.albedo((v + desplazamiento) / largo, u / ancho);
        base.map(|c| (c * tono).clamp(0.0, 1.0))
    }
    fn brillo(&self) -> f64 {
        0.0
    }
}

// Acabado de las cuatro paredes, precalculado una vez y compartido.
pub const ALTURA_MADERA: f64 = 1.10;
pub const COLOR_YESO: Albedo = [0.48, 0.50, 0.38];
const ANCHO_TEXTURA: usize = 512;
const PIXELES_POR_UNIDAD: f64 = 128.0;
const REPETICION_PARED: f64 = 4.0;

struct AcabadoPared {
    colores: std::sync::Arc<Vec<[f32; 3]>>,
    filas: usize,
    altura: f64,
    intercambiar_uv: bool,
}

// Ruido suave, periodico horizontalmente; solo se usa al construir la textura.
fn ruido_yeso(x: f64, y: f64, periodo: i64) -> f64 {
    let hash = |ix: i64, iy: i64| {
        let mut n = (ix.rem_euclid(periodo) as u32).wrapping_mul(374761393)
            ^ (iy as u32).wrapping_mul(668265263);
        n = (n ^ (n >> 13)).wrapping_mul(1274126177);
        (n ^ (n >> 16)) as f64 / u32::MAX as f64
    };
    let ix = x.floor() as i64;
    let iy = y.floor() as i64;
    let fx = x - x.floor();
    let fy = y - y.floor();
    let sx = fx * fx * (3.0 - 2.0 * fx);
    let sy = fy * fy * (3.0 - 2.0 * fy);
    let a = hash(ix, iy) * (1.0 - sx) + hash(ix + 1, iy) * sx;
    let b = hash(ix, iy + 1) * (1.0 - sx) + hash(ix + 1, iy + 1) * sx;
    a * (1.0 - sy) + b * sy - 0.5
}

fn generar_acabado() -> (std::sync::Arc<Vec<[f32; 3]>>, usize) {
    let altura = Y_MAX - Y_MIN;
    let filas = (altura * PIXELES_POR_UNIDAD).ceil().max(2.0) as usize;
    let madera = TexturaMadera::new([0.32, 0.17, 0.075], [0.17, 0.075, 0.025], 6.0, 90.0, 0.0);
    let mut colores = Vec::with_capacity(ANCHO_TEXTURA * filas);
    for y in 0..filas {
        let h = (y as f64 + 0.5) * altura / filas as f64;
        for x in 0..ANCHO_TEXTURA {
            let horizontal = (x as f64 + 0.5) * REPETICION_PARED / ANCHO_TEXTURA as f64;
            let tabla = (horizontal / 0.5).floor();
            let local = horizontal.rem_euclid(0.5);
            let color = if h < ALTURA_MADERA {
                let base = madera.albedo(h / 2.0 + tabla * 0.37, local / 0.5);
                // Marcos y biseles pintados: no agregan geometria ni rayos.
                let factor = if h < 0.09 {
                    0.63
                } else if h < 0.105 {
                    1.20
                } else if local < 0.012 || local > 0.488 {
                    0.48
                } else if local < 0.026 {
                    1.20
                } else if local > 0.474 {
                    0.72
                } else {
                    0.94 + tabla * 0.012
                };
                base.map(|c| c * factor)
            } else if h < ALTURA_MADERA + 0.075 {
                let t = (h - ALTURA_MADERA) / 0.075;
                let factor = if t < 0.15 {
                    0.55
                } else if t > 0.80 {
                    1.30
                } else {
                    0.90
                };
                [0.27 * factor, 0.135 * factor, 0.052 * factor]
            } else {
                let n = ruido_yeso(horizontal * 2.0, h * 2.0, 8) * 0.060
                    + ruido_yeso(horizontal * 6.0, h * 6.0, 24) * 0.025
                    + ruido_yeso(horizontal * 16.0, h * 16.0, 64) * 0.012;
                let hash = ((x as u32).wrapping_mul(374761393)
                    ^ (y as u32).wrapping_mul(668265263))
                .wrapping_mul(1274126177);
                let grano = ((hash >> 24) as f64 / 255.0 - 0.5) * 0.012;
                COLOR_YESO.map(|c| c * (1.0 + n) + grano)
            };
            colores.push(color.map(|c| c.clamp(0.0, 1.0) as f32));
        }
    }
    (std::sync::Arc::new(colores), filas)
}

impl Textura for AcabadoPared {
    fn albedo(&self, u: f64, v: f64) -> Albedo {
        // Paredes X usan UV=(altura,z); paredes Z usan UV=(x,altura).
        let (horizontal, altura) = if self.intercambiar_uv { (v, u) } else { (u, v) };
        let x = horizontal.rem_euclid(REPETICION_PARED) * ANCHO_TEXTURA as f64 / REPETICION_PARED;
        let y =
            (altura / self.altura * self.filas as f64 - 0.5).clamp(0.0, (self.filas - 1) as f64);
        let x0 = x as usize % ANCHO_TEXTURA;
        let x1 = (x0 + 1) % ANCHO_TEXTURA;
        let y0 = y as usize;
        let y1 = (y0 + 1).min(self.filas - 1);
        let fx = (x - x.floor()) as f32;
        let fy = (y - y0 as f64) as f32;
        let a = self.colores[y0 * ANCHO_TEXTURA + x0];
        let b = self.colores[y0 * ANCHO_TEXTURA + x1];
        let c = self.colores[y1 * ANCHO_TEXTURA + x0];
        let d = self.colores[y1 * ANCHO_TEXTURA + x1];
        std::array::from_fn(|i| {
            let arriba = a[i] + (b[i] - a[i]) * fx;
            let abajo = c[i] + (d[i] - c[i]) * fx;
            (arriba + (abajo - arriba) * fy) as f64
        })
    }
}

// Puerta pintada solo sobre la pared del fondo (Z_MIN).
pub const COLOR_TECHO: Albedo = [0.30, 0.31, 0.29];
pub const PUERTA_CENTRO_X: f64 = 3.5;
pub const PUERTA_ANCHO: f64 = 1.90; // Incluye marco.
pub const PUERTA_ALTO: f64 = 3.40;
const PUERTA_TEX_ANCHO: usize = 256;
const PUERTA_TEX_ALTO: usize = 512;

struct ParedConPuerta {
    fondo: Box<dyn Textura>,
    colores: Vec<[u8; 3]>,
}

impl ParedConPuerta {
    fn new(fondo: Box<dyn Textura>) -> Self {
        let madera = TexturaMadera::new([0.45, 0.24, 0.10], [0.23, 0.10, 0.035], 6.0, 90.0, 0.0);
        let mut colores = Vec::with_capacity(PUERTA_TEX_ANCHO * PUERTA_TEX_ALTO);
        for y in 0..PUERTA_TEX_ALTO {
            let v = (y as f64 + 0.5) / PUERTA_TEX_ALTO as f64;
            for x in 0..PUERTA_TEX_ANCHO {
                let u = (x as f64 + 0.5) / PUERTA_TEX_ANCHO as f64;
                let mut c = madera.albedo(v, u);
                let borde = u.min(1.0 - u).min(1.0 - v);
                if borde < 0.012 || v < 0.009 {
                    c = [0.045, 0.023, 0.012];
                } else if borde < 0.065 {
                    let factor = if borde < 0.024 { 1.28 } else { 0.67 };
                    c = c.map(|a| a * factor);
                } else if borde < 0.074 {
                    c = [0.055, 0.025, 0.010];
                } else {
                    // Cuatro paneles con biseles dibujados en la textura.
                    for (x0, x1) in [(0.14, 0.46), (0.54, 0.86)] {
                        for (y0, y1) in [(0.10, 0.43), (0.53, 0.90)] {
                            if u > x0 && u < x1 && v > y0 && v < y1 {
                                let d = (u - x0).min(x1 - u).min(v - y0).min(y1 - v);
                                let factor = if d < 0.009 {
                                    0.45
                                } else if d < 0.025 {
                                    if u - x0 < 0.025 || y1 - v < 0.025 {
                                        1.25
                                    } else {
                                        0.65
                                    }
                                } else {
                                    0.82
                                };
                                c = c.map(|a| a * factor);
                            }
                        }
                    }
                    // Placa y manija laton: color solamente, sin reflejos trazados.
                    if u > 0.865 && u < 0.91 && v > 0.425 && v < 0.505 {
                        c = [0.34, 0.24, 0.08];
                    }
                    if u > 0.785 && u < 0.90 && v > 0.461 && v < 0.476 {
                        c = if v > 0.470 {
                            [0.80, 0.63, 0.29]
                        } else {
                            [0.49, 0.33, 0.10]
                        };
                    }
                }
                colores.push(c.map(|a| a.clamp(0.0, 1.0) as f32));
            }
        }
        // RGB compacto: una lectura por pixel, sin interpolacion por frame.
        let colores = colores
            .into_iter()
            .map(|c| c.map(|v| (v * 255.0).round() as u8))
            .collect();
        Self { fondo, colores }
    }
}

impl Textura for ParedConPuerta {
    fn albedo(&self, u: f64, v: f64) -> Albedo {
        let x = u + X_MIN - (PUERTA_CENTRO_X - PUERTA_ANCHO * 0.5);
        if x < 0.0 || x >= PUERTA_ANCHO || v < 0.0 || v >= PUERTA_ALTO {
            return self.fondo.albedo(u, v);
        }
        let px = (x * ((PUERTA_TEX_ANCHO) as f64 / PUERTA_ANCHO)) as usize;
        let py = (v * ((PUERTA_TEX_ALTO) as f64 / PUERTA_ALTO)) as usize;
        self.colores[py.min(PUERTA_TEX_ALTO - 1) * PUERTA_TEX_ANCHO + px.min(PUERTA_TEX_ANCHO - 1)]
            .map(|c| c as f64 * (1.0 / 255.0))
    }
}

pub struct Habitacion {
    paneles: Vec<Panel>,
}

impl Habitacion {
    pub fn intersectar(&self, ray: &Ray) -> Option<(&dyn Objeto, f64)> {
        let mut cercano: Option<(&dyn Objeto, f64)> = None;
        for panel in &self.paneles {
            if let Some(t) = panel.intersect(ray) {
                if cercano.map(|(_, d)| t < d).unwrap_or(true) {
                    cercano = Some((panel, t));
                }
            }
        }
        cercano
    }
}

/// Interior configurado por los limites compartidos con la camara.
pub fn crear_cuarto() -> Habitacion {
    assert!(MARGEN_CAMARA > 0.0);
    for (min, max) in [(X_MIN, X_MAX), (Y_MIN, Y_MAX), (Z_MIN, Z_MAX)] {
        assert!(
            max - min > 2.0 * MARGEN_CAMARA,
            "habitacion demasiado pequena para el margen de camara"
        );
    }
    let (colores, filas) = generar_acabado();
    let pared = |intercambiar_uv| {
        Box::new(AcabadoPared {
            colores: colores.clone(),
            filas,
            altura: Y_MAX - Y_MIN,
            intercambiar_uv,
        }) as Box<dyn Textura>
    };
    Habitacion {
        paneles: vec![
            Panel::new(
                1,
                Y_MIN,
                [X_MIN, Z_MIN],
                [X_MAX, Z_MAX],
                1.0,
                Box::new(PisoTablas {
                    madera: TexturaMadera::new(
                        [0.47, 0.28, 0.14],
                        [0.29, 0.15, 0.065],
                        6.0,
                        90.0,
                        0.0,
                    ),
                }),
            ),
            Panel::new(
                1,
                Y_MAX,
                [X_MIN, Z_MIN],
                [X_MAX, Z_MAX],
                -1.0,
                Box::new(ColorSolido::new(COLOR_TECHO)),
            ),
            Panel::new(0, X_MIN, [Y_MIN, Z_MIN], [Y_MAX, Z_MAX], 1.0, pared(true)),
            Panel::new(0, X_MAX, [Y_MIN, Z_MIN], [Y_MAX, Z_MAX], -1.0, pared(true)),
            Panel::new(
                2,
                Z_MIN,
                [X_MIN, Y_MIN],
                [X_MAX, Y_MAX],
                1.0,
                Box::new(ParedConPuerta::new(pared(false))),
            ),
            Panel::new(2, Z_MAX, [X_MIN, Y_MIN], [X_MAX, Y_MAX], -1.0, pared(false)),
        ],
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn camara_limites_y_deslizamiento() {
        let mut p = [-1000.0, -1000.0, -1000.0];
        limitar_camara(&mut p);
        assert_eq!(
            p,
            [
                X_MIN + MARGEN_CAMARA,
                Y_MIN + MARGEN_CAMARA,
                Z_MIN + MARGEN_CAMARA
            ]
        );
        p = [1000.0; 3];
        limitar_camara(&mut p);
        assert_eq!(
            p,
            [
                X_MAX - MARGEN_CAMARA,
                Y_MAX - MARGEN_CAMARA,
                Z_MAX - MARGEN_CAMARA
            ]
        );
        p = [1000.0, 2.0, -3.0];
        limitar_camara(&mut p);
        assert_eq!(p, [X_MAX - MARGEN_CAMARA, 2.0, -3.0]);
        p = [0.0, 2.0, 0.0];
        limitar_camara(&mut p);
        assert_eq!(p, [0.0, 2.0, 0.0]);
    }
    #[test]
    fn panel_finito_y_rayo_paralelo() {
        let p = Panel::new(
            1,
            0.0,
            [-1.0, -1.0],
            [1.0, 1.0],
            1.0,
            Box::new(ColorSolido::new([1.0; 3])),
        );
        assert_eq!(
            p.intersect(&Ray::new([0.0, 2.0, 0.0], [0.0, -1.0, 0.0])),
            Some(2.0)
        );
        assert!(
            p.intersect(&Ray::new([2.0, 2.0, 0.0], [0.0, -1.0, 0.0]))
                .is_none()
        );
        assert!(
            p.intersect(&Ray::new([0.0, 2.0, 0.0], [1.0, 0.0, 0.0]))
                .is_none()
        );
        assert!(
            p.intersect(&Ray::new([0.0, 2.0, 0.0], [0.0, 1.0, 0.0]))
                .is_none()
        );
    }
    #[test]
    fn interior_cerrado_normales_hacia_dentro() {
        let h = crear_cuarto();
        for eje in 0..3 {
            for signo in [-1.0, 1.0] {
                let mut direccion = [0.0; 3];
                direccion[eje] = signo;
                let ray = Ray::new([0.0, 1.65, 0.70], direccion);
                let (objeto, t) = h.intersectar(&ray).unwrap();
                assert!(t > 0.0);
                assert_eq!(objeto.normal(ray.point_at(t))[eje], -signo);
            }
        }
    }
}

/// Relleno neutro artistico para el interior; no simula rebotes ni sombras.
pub fn relleno_ambiente(color: raylib::prelude::Color, albedo: Albedo) -> raylib::prelude::Color {
    let canal =
        |base: u8, i: usize| (base as f64 + albedo[i] * 0.24 * 255.0).clamp(0.0, 255.0) as u8;
    raylib::prelude::Color::new(canal(color.r, 0), canal(color.g, 1), canal(color.b, 2), 255)
}

