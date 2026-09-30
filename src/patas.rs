use crate::mesa::CajaRectangular;
use crate::rayIntersect::{Objeto, Ray, normalize};
use crate::textura::{ColorSolido, Textura, TexturaMadera};
use std::f64::consts::TAU;

// Perfil de torno: (altura relativa, radio). Pie, anillos, cuello y hombro.
const PERFIL: [(f64, f64); 13] = [
    (0.00, 0.25),
    (0.07, 0.25),
    (0.13, 0.21),
    (0.18, 0.17),
    (0.23, 0.21),
    (0.28, 0.21),
    (0.34, 0.145),
    (0.62, 0.115),
    (0.70, 0.17),
    (0.75, 0.22),
    (0.82, 0.22),
    (0.89, 0.18),
    (1.0, 0.20),
];

/// Superficie de revolucion por tramos, con descarte por caja antes del perfil.
pub struct PataTorneada {
    x: f64,
    z: f64,
    alto: f64,
    limite: CajaRectangular,
    madera: TexturaMadera,
}

impl PataTorneada {
    pub fn new(x: f64, z: f64, alto: f64) -> Self {
        assert!(alto > 0.0);
        Self {
            x,
            z,
            alto,
            limite: CajaRectangular::new(
                [x, alto * 0.5, z],
                [0.50, alto, 0.50],
                Box::new(ColorSolido::new([0.0; 3])),
            ),
            madera: TexturaMadera::new([0.30, 0.085, 0.025], [0.14, 0.038, 0.010], 6.0, 90.0, 24.0),
        }
    }

    fn tramo(&self, y: f64) -> (f64, f64) {
        for puntos in PERFIL.windows(2) {
            let (h0, r0) = puntos[0];
            let (h1, r1) = puntos[1];
            if y <= h1 * self.alto + 1e-8 {
                let pendiente = (r1 - r0) / ((h1 - h0) * self.alto);
                return (r0 + pendiente * (y - h0 * self.alto), pendiente);
            }
        }
        (0.20, 0.0)
    }
}

impl Objeto for PataTorneada {
    fn intersect(&self, ray: &Ray) -> Option<f64> {
        self.limite.intersect(ray)?;
        let ox = ray.origin[0] - self.x;
        let oz = ray.origin[2] - self.z;
        let [dx, dy, dz] = ray.direction;
        let mut cercano = f64::INFINITY;
        for puntos in PERFIL.windows(2) {
            let (h0, r0) = puntos[0];
            let (h1, r1) = puntos[1];
            let y0 = h0 * self.alto;
            let y1 = h1 * self.alto;
            let k = (r1 - r0) / (y1 - y0);
            let r = r0 + k * (ray.origin[1] - y0);
            let a = dx * dx + dz * dz - (k * dy).powi(2);
            let b = 2.0 * (ox * dx + oz * dz - r * k * dy);
            let c = ox * ox + oz * oz - r * r;
            let mut probar = |t: f64| {
                let y = ray.origin[1] + t * dy;
                if t > 1e-7 && y >= y0 && y <= y1 {
                    cercano = cercano.min(t);
                }
            };
            if a.abs() < 1e-12 {
                if b.abs() > 1e-12 {
                    probar(-c / b);
                }
            } else {
                let disc = b * b - 4.0 * a * c;
                if disc >= 0.0 {
                    let raiz = disc.sqrt();
                    probar((-b - raiz) / (2.0 * a));
                    probar((-b + raiz) / (2.0 * a));
                }
            }
        }
        if dy.abs() > 1e-12 {
            for (y, r) in [(0.0, 0.25), (self.alto, 0.20)] {
                let t = (y - ray.origin[1]) / dy;
                if t > 1e-7 && (ox + t * dx).powi(2) + (oz + t * dz).powi(2) <= r * r {
                    cercano = cercano.min(t);
                }
            }
        }
        cercano.is_finite().then_some(cercano)
    }
    fn normal(&self, p: [f64; 3]) -> [f64; 3] {
        if p[1].abs() < 1e-7 {
            return [0.0, -1.0, 0.0];
        }
        if (p[1] - self.alto).abs() < 1e-7 {
            return [0.0, 1.0, 0.0];
        }
        let (r, k) = self.tramo(p[1]);
        normalize([p[0] - self.x, -r * k, p[2] - self.z])
    }
    fn uv(&self, p: [f64; 3]) -> (f64, f64) {
        (
            p[1] / self.alto,
            ((p[2] - self.z).atan2(p[0] - self.x) / TAU).rem_euclid(1.0),
        )
    }
    fn textura(&self) -> &dyn Textura {
        &self.madera
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn perfil_tapas_y_huecos() {
        let pata = PataTorneada::new(0.0, 0.0, 1.0);
        for (y, radio) in [(0.03, 0.25), (0.25, 0.21), (0.62, 0.115), (0.78, 0.22)] {
            let t = pata
                .intersect(&Ray::new([1.0, y, 0.0], [-1.0, 0.0, 0.0]))
                .unwrap();
            assert!((t - (1.0 - radio)).abs() < 1e-8);
        }
        assert_eq!(
            pata.intersect(&Ray::new([0.0, 2.0, 0.0], [0.0, -1.0, 0.0])),
            Some(1.0)
        );
        assert!(
            pata.intersect(&Ray::new([0.3, 2.0, 0.0], [0.0, -1.0, 0.0]))
                .is_none()
        );
        assert!(
            pata.intersect(&Ray::new([0.0, 0.5, 0.0], [1.0, 0.0, 0.0]))
                .unwrap()
                > 0.0
        );
    }
}