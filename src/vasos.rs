use crate::rayIntersect::{Objeto, Ray, normalize};
use crate::textura::{Albedo, Textura};

pub const RADIO: f64 = 0.11;
pub const ALTO: f64 = 0.32;
const PARED: f64 = 0.009;
const FONDO: f64 = 0.035;
const EPS: f64 = 1e-6;

/// Cilindro cerrado de liquido o recipiente hueco con fondo y borde anular.
pub struct Vaso {
    centro: [f64; 3],
    radio: f64,
    alto: f64,
    hueco: bool,
    material: MaterialVaso,
}

impl Vaso {
    pub fn vidrio(centro: [f64; 3]) -> Self {
        Self {
            centro,
            radio: RADIO,
            alto: ALTO,
            hueco: true,
            material: MaterialVaso { liquido: false },
        }
    }
    pub fn liquido(mut centro: [f64; 3]) -> Self {
        centro[1] += FONDO + 0.0001;
        Self {
            centro,
            radio: RADIO - PARED - 0.0001,
            alto: 0.145,
            hueco: false,
            material: MaterialVaso { liquido: true },
        }
    }
    pub fn limites(&self) -> ([f64; 3], [f64; 3]) {
        (
            [
                self.centro[0],
                self.centro[1] + self.alto * 0.5,
                self.centro[2],
            ],
            [2.0 * self.radio, self.alto, 2.0 * self.radio],
        )
    }
}

impl Objeto for Vaso {
    fn intersect(&self, ray: &Ray) -> Option<f64> {
        let ox = ray.origin[0] - self.centro[0];
        let oz = ray.origin[2] - self.centro[2];
        let oy = ray.origin[1] - self.centro[1];
        let [dx, dy, dz] = ray.direction;
        let a = dx * dx + dz * dz;
        let b = ox * dx + oz * dz;
        let mut hit = f64::INFINITY;
        if a > 1e-14 {
            let lados = if self.hueco { 2 } else { 1 };
            for i in 0..lados {
                let r = self.radio - if i == 1 { PARED } else { 0.0 };
                let ymin = if i == 1 { FONDO } else { 0.0 };
                let disc = b * b - a * (ox * ox + oz * oz - r * r);
                if disc >= 0.0 {
                    let raiz = disc.sqrt();
                    for t in [(-b - raiz) / a, (-b + raiz) / a] {
                        let y = oy + t * dy;
                        if t > EPS && y >= ymin && y <= self.alto {
                            hit = hit.min(t);
                        }
                    }
                }
            }
        }
        if dy.abs() > 1e-12 {
            let tapas = if self.hueco { 3 } else { 2 };
            for i in 0..tapas {
                let y = match i {
                    0 => 0.0,
                    1 => self.alto,
                    _ => FONDO,
                };
                let t = (y - oy) / dy;
                if t <= EPS {
                    continue;
                }
                let r2 = (ox + t * dx).powi(2) + (oz + t * dz).powi(2);
                let exterior = if i == 2 {
                    self.radio - PARED
                } else {
                    self.radio
                };
                let interior = if self.hueco && i == 1 {
                    self.radio - PARED
                } else {
                    0.0
                };
                if r2 <= exterior * exterior && r2 >= interior * interior {
                    hit = hit.min(t);
                }
            }
        }
        hit.is_finite().then_some(hit)
    }
    fn normal(&self, p: [f64; 3]) -> [f64; 3] {
        let y = p[1] - self.centro[1];
        if y.abs() < EPS {
            return [0.0, -1.0, 0.0];
        }
        if (y - self.alto).abs() < EPS || (self.hueco && (y - FONDO).abs() < EPS) {
            return [0.0, 1.0, 0.0];
        }
        let x = p[0] - self.centro[0];
        let z = p[2] - self.centro[2];
        let radio = (x * x + z * z).sqrt();
        let signo =
            if self.hueco && (radio - (self.radio - PARED)).abs() < (radio - self.radio).abs() {
                -1.0
            } else {
                1.0
            };
        normalize([x * signo, 0.0, z * signo])
    }
    fn uv(&self, p: [f64; 3]) -> (f64, f64) {
        (0.0, (p[1] - self.centro[1]) / self.alto)
    }
    fn textura(&self) -> &dyn Textura {
        &self.material
    }
}

struct MaterialVaso {
    liquido: bool,
}
impl Textura for MaterialVaso {
    fn albedo(&self, _: f64, _: f64) -> Albedo {
        if self.liquido {
            [1.0, 0.48, 0.045]
        } else {
            [0.83, 0.91, 0.98]
        }
    }
    fn brillo(&self) -> f64 {
        if self.liquido { 90.0 } else { 160.0 }
    }
    fn transparencia(&self) -> f64 {
        if self.liquido { 0.68 } else { 0.96 }
    }
    fn filtro(&self) -> Albedo {
        if self.liquido {
            [1.0, 0.78, 0.30]
        } else {
            [0.99, 0.995, 1.0]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn vaso_abierto_fondo_y_pared_interior() {
        let v = Vaso::vidrio([0.0; 3]);
        let t = v
            .intersect(&Ray::new([0.0, 1.0, 0.0], [0.0, -1.0, 0.0]))
            .unwrap();
        assert!((t - (1.0 - FONDO)).abs() < 1e-8);
        let r = Ray::new([0.0, 0.2, 0.0], [1.0, 0.0, 0.0]);
        let t = v.intersect(&r).unwrap();
        assert!((t - (RADIO - PARED)).abs() < 1e-8);
        assert_eq!(v.normal(r.point_at(t)), [-1.0, 0.0, 0.0]);
        let l = Vaso::liquido([0.0; 3]);
        let t = l
            .intersect(&Ray::new([0.0, 1.0, 0.0], [0.0, -1.0, 0.0]))
            .unwrap();
        assert!((t - (1.0 - FONDO - 0.0001 - 0.145)).abs() < 1e-8);
    }
}
