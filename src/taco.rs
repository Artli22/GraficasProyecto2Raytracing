use crate::rayIntersect::{Objeto, Ray, cross, dot, escala, normalize, sub, suma};
use crate::textura::{Albedo, Textura, TexturaMadera};
use std::f64::consts::TAU;

const EPS: f64 = 1e-7;

/// Un solo tronco de cono cerrado: evita probar varios cilindros por rayo.
pub struct Taco {
    inicio: [f64; 3],
    eje: [f64; 3],
    lateral: [f64; 3],
    arriba: [f64; 3],
    largo: f64,
    radio: f64,
    pendiente: f64,
    material: MaderaTaco,
}

impl Taco {
    pub fn new(inicio: [f64; 3], fin: [f64; 3], radio: f64, radio_punta: f64) -> Self {
        let delta = sub(fin, inicio);
        let largo = dot(delta, delta).sqrt();
        assert!(largo > EPS && radio > 0.0 && radio_punta > 0.0);
        let eje = escala(delta, 1.0 / largo);
        let referencia = if eje[1].abs() < 0.99 {
            [0.0, 1.0, 0.0]
        } else {
            [1.0, 0.0, 0.0]
        };
        let lateral = normalize(cross(eje, referencia));
        let arriba = cross(lateral, eje);
        Self {
            inicio,
            eje,
            lateral,
            arriba,
            largo,
            radio,
            pendiente: (radio_punta - radio) / largo,
            material: MaderaTaco {
                // Reutiliza el generador de madera existente, con tonos claros.
                madera: TexturaMadera::new(
                    [0.72, 0.43, 0.20],
                    [0.44, 0.23, 0.085],
                    6.0,
                    90.0,
                    40.0,
                ),
            },
        }
    }

    fn local(&self, punto: [f64; 3]) -> (f64, [f64; 3]) {
        let q = sub(punto, self.inicio);
        let z = dot(q, self.eje);
        (z, sub(q, escala(self.eje, z)))
    }
}

impl Objeto for Taco {
    fn intersect(&self, ray: &Ray) -> Option<f64> {
        let (z, q) = self.local(ray.origin);
        let dz = dot(ray.direction, self.eje);
        let d = sub(ray.direction, escala(self.eje, dz));
        let r = self.radio + self.pendiente * z;
        let dr = self.pendiente * dz;
        let a = dot(d, d) - dr * dr;
        let b = 2.0 * (dot(q, d) - r * dr);
        let c = dot(q, q) - r * r;
        let mut cercano = f64::INFINITY;
        let mut probar_lado = |t: f64| {
            let altura = z + t * dz;
            if t > EPS && altura >= 0.0 && altura <= self.largo {
                cercano = cercano.min(t);
            }
        };
        if a.abs() < 1e-12 {
            if b.abs() > 1e-12 {
                probar_lado(-c / b);
            }
        } else {
            let discriminante = b * b - 4.0 * a * c;
            if discriminante >= 0.0 {
                let raiz = discriminante.sqrt();
                probar_lado((-b - raiz) / (2.0 * a));
                probar_lado((-b + raiz) / (2.0 * a));
            }
        }
        // Tapas del mango y la punta; también admite rayos desde dentro.
        if dz.abs() > 1e-12 {
            for altura in [0.0, self.largo] {
                let t = (altura - z) / dz;
                let radial = suma(q, escala(d, t));
                let radio = self.radio + self.pendiente * altura;
                if t > EPS && dot(radial, radial) <= radio * radio {
                    cercano = cercano.min(t);
                }
            }
        }
        cercano.is_finite().then_some(cercano)
    }

    fn normal(&self, punto: [f64; 3]) -> [f64; 3] {
        let (z, radial) = self.local(punto);
        if z.abs() < EPS {
            return escala(self.eje, -1.0);
        }
        if (z - self.largo).abs() < EPS {
            return self.eje;
        }
        let r = self.radio + self.pendiente * z;
        normalize(sub(radial, escala(self.eje, self.pendiente * r)))
    }

    fn uv(&self, punto: [f64; 3]) -> (f64, f64) {
        let (z, radial) = self.local(punto);
        let angulo = dot(radial, self.arriba).atan2(dot(radial, self.lateral));
        (
            (z / self.largo).clamp(0.0, 1.0),
            (angulo / TAU).rem_euclid(1.0),
        )
    }

    fn textura(&self) -> &dyn Textura {
        &self.material
    }
}

struct MaderaTaco {
    madera: TexturaMadera,
}

impl Textura for MaderaTaco {
    fn albedo(&self, u: f64, v: f64) -> Albedo {
        if u < 0.018 {
            return [0.025, 0.020, 0.018];
        } // Tope de goma.
        if u > 0.985 {
            return [0.12, 0.48, 0.64];
        } // Punta con tiza azul.
        if u > 0.957 {
            return [0.92, 0.87, 0.70];
        } // Virola marfil.
        // Anillos oscuros y finas incrustaciones doradas en el mango.
        for centro in [0.045, 0.085, 0.285, 0.325] {
            let distancia = (u - centro).abs();
            if distancia < 0.004 {
                return [0.85, 0.63, 0.25];
            }
            if distancia < 0.012 {
                return [0.085, 0.035, 0.018];
            }
        }
        // Cuatro rombos alrededor del extremo grueso del taco.
        let alrededor = ((v * 4.0).fract() - 0.5).abs() / 0.34;
        let rombo = (u - 0.185).abs() / 0.073 + alrededor;
        if rombo < 0.74 {
            return [0.91, 0.79, 0.51];
        }
        if rombo < 1.0 {
            return [0.11, 0.045, 0.02];
        }
        self.madera.albedo(u, v)
    }
    fn brillo(&self) -> f64 {
        40.0
    }
}

/// A la izquierda de las bolas; la cara inferior queda sobre el pano.
pub fn crear_taco() -> Taco {
    Taco::new(
        [-1.05, crate::mesa::ALTURA_PANO + 0.049, -2.00],
        [-0.85, crate::mesa::ALTURA_PANO + 0.021, -5.95],
        0.048,
        0.020,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    fn cilindro() -> Taco {
        Taco::new([0.0, 0.0, 0.0], [0.0, 0.0, 2.0], 0.5, 0.5)
    }
    fn cerca(a: f64, b: f64) {
        assert!((a - b).abs() < 1e-8, "{a} != {b}");
    }
    #[test]
    fn lados_tapas_y_origen_interior() {
        let t = cilindro();
        cerca(
            t.intersect(&Ray::new([2.0, 0.0, 1.0], [-1.0, 0.0, 0.0]))
                .unwrap(),
            1.5,
        );
        cerca(
            t.intersect(&Ray::new([0.0, 0.0, -1.0], [0.0, 0.0, 1.0]))
                .unwrap(),
            1.0,
        );
        cerca(
            t.intersect(&Ray::new([0.0, 0.0, 3.0], [0.0, 0.0, -1.0]))
                .unwrap(),
            1.0,
        );
        cerca(
            t.intersect(&Ray::new([0.0, 0.0, 1.0], [1.0, 0.0, 0.0]))
                .unwrap(),
            0.5,
        );
        assert!(
            t.intersect(&Ray::new([2.0, 0.0, 3.0], [-1.0, 0.0, 0.0]))
                .is_none()
        );
    }
    #[test]
    fn cono_radio_variable_y_normal() {
        let t = Taco::new([0.0, 0.0, 0.0], [0.0, 0.0, 2.0], 0.5, 0.1);
        cerca(
            t.intersect(&Ray::new([2.0, 0.0, 1.0], [-1.0, 0.0, 0.0]))
                .unwrap(),
            1.7,
        );
        let n = t.normal([0.3, 0.0, 1.0]);
        cerca(dot(n, n), 1.0);
        assert!(n[0] > 0.0 && n[2] > 0.0);
    }
}
