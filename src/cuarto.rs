use crate::rayIntersect::{Objeto, Ray};
use crate::textura::{Albedo, ColorSolido, Textura, TexturaMadera};

const EPS: f64 = 1e-7;

// Edita estos limites para cambiar el tamano: geometria y camara los comparten.
pub const X_MIN: f64 = -8.0;
pub const X_MAX: f64 = 8.0;
pub const Y_MIN: f64 = 0.0; // Piso.
pub const Y_MAX: f64 = 7.0; // Techo.
pub const Z_MIN: f64 = -14.0;
pub const Z_MAX: f64 = 6.0;
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
pub fn crear_habitacion() -> Habitacion {
    assert!(MARGEN_CAMARA > 0.0);
    for (min, max) in [(X_MIN, X_MAX), (Y_MIN, Y_MAX), (Z_MIN, Z_MAX)] {
        assert!(
            max - min > 2.0 * MARGEN_CAMARA,
            "habitacion demasiado pequena para el margen de camara"
        );
    }
    let gris = || Box::new(ColorSolido::new([0.56, 0.55, 0.52])) as Box<dyn Textura>;
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
                Box::new(ColorSolido::new([0.83, 0.82, 0.78])),
            ),
            Panel::new(0, X_MIN, [Y_MIN, Z_MIN], [Y_MAX, Z_MAX], 1.0, gris()),
            Panel::new(0, X_MAX, [Y_MIN, Z_MIN], [Y_MAX, Z_MAX], -1.0, gris()),
            Panel::new(2, Z_MIN, [X_MIN, Y_MIN], [X_MAX, Y_MAX], 1.0, gris()),
            Panel::new(2, Z_MAX, [X_MIN, Y_MIN], [X_MAX, Y_MAX], -1.0, gris()),
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
        let h = crear_habitacion();
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
