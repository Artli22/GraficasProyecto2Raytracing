use crate::rayIntersect::{Objeto, PasoOptico, Ray, dot, escala, normalize, sub};
use crate::textura::{Albedo, Textura, sombrear_puntual};

pub const RADIO: f64 = 0.11;
pub const ALTO: f64 = 0.32;
pub const INDICE_LIQUIDO: f64 = 1.33;
const FONDO: f64 = 0.035;
const RADIO_LIQUIDO: f64 = RADIO - 0.0091;
const NIVEL: f64 = FONDO + 0.1451;
const EPS: f64 = 1e-6;
// Absorcion por unidad recorrida: el azul se absorbe mas que el rojo.
const ABSORCION: [f64; 3] = [0.65, 4.0, 17.0];

/// Snell, con normal orientada contra la direccion incidente. None indica RIT.
fn refractar(d: [f64; 3], n: [f64; 3], eta: f64) -> Option<[f64; 3]> {
    let cos = (-dot(d, n)).clamp(0.0, 1.0);
    let k = 1.0 - eta * eta * (1.0 - cos * cos);
    if k < 0.0 {
        None
    } else {
        Some(normalize(std::array::from_fn(|i| {
            eta * d[i] + (eta * cos - k.sqrt()) * n[i]
        })))
    }
}
fn fresnel(cos: f64) -> f64 {
    let r0 = ((INDICE_LIQUIDO - 1.0) / (INDICE_LIQUIDO + 1.0)).powi(2);
    r0 + (1.0 - r0) * (1.0 - cos.clamp(0.0, 1.0)).powi(5)
}

/// Unico objeto del BVH: envolvente del vaso e interior optico analitico.
pub struct Vaso {
    centro: [f64; 3],
    material: MaterialVaso,
}
impl Vaso {
    pub fn new(centro: [f64; 3]) -> Self {
        Self {
            centro,
            material: MaterialVaso,
        }
    }
    pub fn limites(&self) -> ([f64; 3], [f64; 3]) {
        (
            [self.centro[0], self.centro[1] + ALTO * 0.5, self.centro[2]],
            [RADIO * 2.0, ALTO, RADIO * 2.0],
        )
    }
    // Intervalo de un cilindro cerrado: una cuadratica y dos planos horizontales.
    fn intervalo(&self, ray: &Ray, radio: f64, bajo: f64, alto: f64) -> Option<(f64, f64)> {
        let x = ray.origin[0] - self.centro[0];
        let z = ray.origin[2] - self.centro[2];
        let y = ray.origin[1] - self.centro[1];
        let [dx, dy, dz] = ray.direction;
        let a = dx * dx + dz * dz;
        let b = x * dx + z * dz;
        let c = x * x + z * z - radio * radio;
        let (mut entrada, mut salida) = (f64::NEG_INFINITY, f64::INFINITY);
        if a < 1e-16 {
            if c > 0.0 {
                return None;
            }
        } else {
            let disc = b * b - a * c;
            if disc < 0.0 {
                return None;
            }
            let raiz = disc.sqrt();
            entrada = (-b - raiz) / a;
            salida = (-b + raiz) / a;
        }
        if dy.abs() < 1e-12 {
            if y < bajo || y > alto {
                return None;
            }
        } else {
            let a = (bajo - y) / dy;
            let b = (alto - y) / dy;
            entrada = entrada.max(a.min(b));
            salida = salida.min(a.max(b));
        }
        if salida < entrada || salida <= EPS {
            None
        } else {
            Some((entrada, salida))
        }
    }
    fn normal_cilindro(&self, p: [f64; 3], bajo: f64, alto: f64) -> [f64; 3] {
        let y = p[1] - self.centro[1];
        if (y - bajo).abs() < EPS * 4.0 {
            return [0.0, -1.0, 0.0];
        }
        if (y - alto).abs() < EPS * 4.0 {
            return [0.0, 1.0, 0.0];
        }
        normalize([p[0] - self.centro[0], 0.0, p[2] - self.centro[2]])
    }
}
impl Objeto for Vaso {
    fn intersect(&self, ray: &Ray) -> Option<f64> {
        let (a, b) = self.intervalo(ray, RADIO, 0.0, ALTO)?;
        Some(if a > EPS { a } else { b })
    }
    fn normal(&self, p: [f64; 3]) -> [f64; 3] {
        self.normal_cilindro(p, 0.0, ALTO)
    }
    fn uv(&self, p: [f64; 3]) -> (f64, f64) {
        (0.0, (p[1] - self.centro[1]) / ALTO)
    }
    fn textura(&self) -> &dyn Textura {
        &self.material
    }
    fn atravesar(&self, ray: &Ray, t: f64) -> Option<PasoOptico> {
        let p = ray.point_at(t);
        let n = self.normal(p);
        let desde_dentro = self
            .intervalo(ray, RADIO, 0.0, ALTO)
            .map(|(a, _)| a <= EPS)
            .unwrap_or(false);
        let r2 = (p[0] - self.centro[0]).powi(2) + (p[2] - self.centro[2]).powi(2);
        let abertura =
            (p[1] - self.centro[1] - ALTO).abs() < EPS * 4.0 && r2 < RADIO_LIQUIDO.powi(2);
        let borde = (1.0 - dot(n, ray.direction).abs().clamp(0.0, 1.0)).powi(5);
        let alpha = if abertura { 0.0 } else { 0.035 + 0.60 * borde };
        let brillo = sombrear_puntual(
            ray.direction,
            p,
            n,
            crate::lampara::LUZ_POSICION,
            [1.0; 3],
            [0.83, 0.91, 0.98],
            160.0,
        );
        let colores = [brillo.r, brillo.g, brillo.b];
        let mut aporte =
            std::array::from_fn(|i| alpha * (colores[i] as f64 / 255.0 + 0.28).min(1.0));
        let mut filtro = [1.0 - alpha; 3];
        let mut actual = Ray {
            origin: if desde_dentro {
                ray.origin
            } else {
                ray.point_at(t + EPS * 8.0)
            },
            direction: ray.direction,
        };
        if let Some((entrada, salida)) = self.intervalo(&actual, RADIO_LIQUIDO, FONDO, NIVEL) {
            if salida > EPS {
                let dentro = entrada <= EPS;
                let entrada_p = actual.point_at(entrada.max(0.0));
                let normal_entrada = self.normal_cilindro(entrada_p, FONDO, NIVEL);
                let energia_entrada = if dentro {
                    1.0
                } else {
                    1.0 - fresnel(-dot(actual.direction, normal_entrada))
                };
                if !dentro {
                    let dir = refractar(actual.direction, normal_entrada, 1.0 / INDICE_LIQUIDO)?;
                    actual = Ray {
                        origin: std::array::from_fn(|i| entrada_p[i] + dir[i] * EPS * 8.0),
                        direction: dir,
                    };
                }
                let mut distancia = 0.0;
                let mut transmitido = false;
                let mut energia_salida = 1.0;
                // Una reflexion interna total como maximo, sin bifurcar rayos.
                for intento in 0..2 {
                    let Some((_, hasta)) = self.intervalo(&actual, RADIO_LIQUIDO, FONDO, NIVEL)
                    else {
                        break;
                    };
                    distancia += hasta;
                    let q = actual.point_at(hasta);
                    let normal = self.normal_cilindro(q, FONDO, NIVEL);
                    let cos = dot(actual.direction, normal).clamp(0.0, 1.0);
                    if let Some(dir) =
                        refractar(actual.direction, escala(normal, -1.0), INDICE_LIQUIDO)
                    {
                        energia_salida = 1.0 - fresnel(cos);
                        actual = Ray {
                            origin: std::array::from_fn(|i| q[i] + dir[i] * EPS * 8.0),
                            direction: dir,
                        };
                        transmitido = true;
                        break;
                    }
                    if intento == 0 {
                        let dir = normalize(sub(
                            actual.direction,
                            escala(normal, 2.0 * dot(actual.direction, normal)),
                        ));
                        actual = Ray {
                            origin: std::array::from_fn(|i| q[i] + dir[i] * EPS * 8.0),
                            direction: dir,
                        };
                    }
                }
                let tono = [1.0, 0.34, 0.025];
                // Aporte artistico de color dispersado: conserva el ambar ante fondos verdes.
                let dispersion = (1.0 - (-6.0 * distancia).exp()) * 0.55;
                // Beer-Lambert: mas recorrido en liquido, mas absorcion.
                for i in 0..3 {
                    let transmision = (-ABSORCION[i] * distancia).exp();
                    aporte[i] += filtro[i] * dispersion * tono[i];
                    filtro[i] *= transmision * energia_entrada * energia_salida;
                    if !transmitido {
                        filtro[i] = 0.0;
                    }
                }
            }
        }
        // Salta todo el recipiente antes de la siguiente consulta global.
        if let Some((_, salida)) = self.intervalo(&actual, RADIO, 0.0, ALTO) {
            actual.origin = actual.point_at(salida + EPS * 8.0);
        }
        Some(PasoOptico {
            rayo: actual,
            filtro,
            aporte,
        })
    }
}
struct MaterialVaso;
impl Textura for MaterialVaso {
    fn albedo(&self, _: f64, _: f64) -> Albedo {
        [0.83, 0.91, 0.98]
    }
    fn brillo(&self) -> f64 {
        160.0
    }
    fn transparencia(&self) -> f64 {
        0.96
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn solo_dos_consultas_globales_para_un_vaso() {
        let v = Vaso::new([0.0; 3]);
        let ray = Ray::new([0.0, 0.1, -1.0], [0.0, 0.0, 1.0]);
        let consultas = std::cell::Cell::new(0);
        let _ = crate::render::trazar(&ray, &|r| {
            consultas.set(consultas.get() + 1);
            v.intersect(r).map(|t| (&v as &dyn Objeto, t, false))
        });
        assert_eq!(consultas.get(), 2);
    }
    #[test]
    fn snell_normal_oblicua_y_reflexion_interna() {
        assert_eq!(
            refractar([0.0, -1.0, 0.0], [0.0, 1.0, 0.0], 1.0 / 1.33).unwrap(),
            [0.0, -1.0, 0.0]
        );
        let d = normalize([1.0, -1.0, 0.0]);
        let r = refractar(d, [0.0, 1.0, 0.0], 1.0 / 1.33).unwrap();
        assert!((r[0] - d[0] / 1.33).abs() < 1e-10);
        assert!(refractar(normalize([1.0, -0.1, 0.0]), [0.0, 1.0, 0.0], 1.33).is_none());
    }
    #[test]
    fn sale_del_vaso_y_absorbe_azul() {
        let v = Vaso::new([0.0; 3]);
        for (o, d) in [
            ([-1.0, 0.1, 0.03], [1.0, 0.0, 0.0]),
            ([0.0, 1.0, 0.0], [0.0, -1.0, 0.0]),
            ([0.0, 0.1, 0.0], [1.0, 0.0, 0.0]),
        ] {
            let ray = Ray::new(o, d);
            let t = v.intersect(&ray).unwrap();
            let paso = v.atravesar(&ray, t).unwrap();
            assert!(paso.filtro[0] > paso.filtro[2]);
            assert!(
                paso.rayo
                    .origin
                    .iter()
                    .chain(paso.rayo.direction.iter())
                    .all(|x| x.is_finite())
            );
            assert!(v.intersect(&paso.rayo).is_none());
        }
    }
    #[test]
    fn parte_vacia_no_tine_y_liquido_desvia() {
        let v = Vaso::new([0.0; 3]);
        let ray = Ray::new([-1.0, 0.26, 0.04], [1.0, 0.0, 0.0]);
        let p = v.atravesar(&ray, v.intersect(&ray).unwrap()).unwrap();
        assert_eq!(p.rayo.direction, ray.direction);
        assert!((p.filtro[0] - p.filtro[2]).abs() < 1e-12);
        let ray = Ray::new([-1.0, 0.10, 0.04], [1.0, 0.0, 0.0]);
        let p = v.atravesar(&ray, v.intersect(&ray).unwrap()).unwrap();
        assert!(p.rayo.direction[2].abs() > 0.01);
    }
}
