//! Cubo de profundidad inmutable alrededor de una luz puntual fija.
//! Solo usa std. Reconstruir si cambia la geometria opaca o la luz.
use crate::rayIntersect::{Ray, dot, sub};

pub const RESOLUCION: usize = 1024;
/// Fraccion de luz directa retirada en una sombra completa (0..1).
pub const INTENSIDAD_OBJETOS: f64 = 0.60;
pub const INTENSIDAD_PAREDES: f64 = 0.25;
pub const INTENSIDAD_PISO: f64 = 0.45;

pub struct MapaSombras {
    luz: [f64; 3],
    lado: usize,
    profundidad: Vec<f32>,
}

// Cara = eje mayor * 2 + signo negativo. Los otros ejes son ciclicos.
fn direccion(cara: usize, u: f64, v: f64) -> [f64; 3] {
    let eje = cara / 2;
    let mut d = [0.0; 3];
    d[eje] = if cara % 2 == 0 { 1.0 } else { -1.0 };
    d[(eje + 1) % 3] = u;
    d[(eje + 2) % 3] = v;
    d
}

impl MapaSombras {
    /// `distancia` devuelve el primer obstaculo opaco desde la luz.
    pub fn construir<F>(luz: [f64; 3], lado: usize, distancia: &F) -> Self
    where
        F: Fn(&Ray) -> Option<f64> + Sync,
    {
        assert!(lado >= 2);
        let total = lado
            .checked_mul(lado)
            .and_then(|n| n.checked_mul(6))
            .expect("mapa demasiado grande");
        let mut profundidad = vec![f32::INFINITY; total];
        let hilos = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1)
            .min(16);
        let bloque = total.div_ceil(hilos);
        std::thread::scope(|s| {
            for (k, datos) in profundidad.chunks_mut(bloque).enumerate() {
                s.spawn(move || {
                    for (j, valor) in datos.iter_mut().enumerate() {
                        let i = k * bloque + j;
                        let cara = i / (lado * lado);
                        let x = i % lado;
                        let y = (i / lado) % lado;
                        let d = direccion(
                            cara,
                            2.0 * (x as f64 + 0.5) / lado as f64 - 1.0,
                            2.0 * (y as f64 + 0.5) / lado as f64 - 1.0,
                        );
                        let r = Ray::new(luz, d);
                        if let Some(t) = distancia(&r) {
                            if t > 0.0 {
                                *valor = (t * r.direction[cara / 2].abs()) as f32;
                            }
                        }
                    }
                });
            }
        });
        Self {
            luz,
            lado,
            profundidad,
        }
    }

    /// Cuatro comparaciones con interpolacion. Corrige profundidad usando el
    /// plano tangente del receptor para evitar acne en piso y paredes oblicuos.
    pub fn visibilidad(&self, p: [f64; 3], n: [f64; 3], cuarto: bool) -> f64 {
        let delta = sub(p, self.luz);
        let plano = dot(n, delta);
        if plano >= 0.0 {
            return 0.0;
        }
        let mut eje = 0;
        for i in 1..3 {
            if delta[i].abs() > delta[eje].abs() {
                eje = i;
            }
        }
        let mayor = delta[eje].abs();
        if mayor < 1e-8 {
            return 1.0;
        }
        let cara = eje * 2 + usize::from(delta[eje] < 0.0);
        let u = delta[(eje + 1) % 3] / mayor;
        let v = delta[(eje + 2) % 3] / mayor;
        let x = ((u + 1.0) * 0.5 * self.lado as f64 - 0.5).clamp(0.0, (self.lado - 1) as f64);
        let y = ((v + 1.0) * 0.5 * self.lado as f64 - 0.5).clamp(0.0, (self.lado - 1) as f64);
        let x0 = x as usize;
        let y0 = y as usize;
        let fx = x - x0 as f64;
        let fy = y - y0 as f64;
        let mut visible = 0.0;
        for (yy, wy) in [(y0, 1.0 - fy), ((y0 + 1).min(self.lado - 1), fy)] {
            for (xx, wx) in [(x0, 1.0 - fx), ((x0 + 1).min(self.lado - 1), fx)] {
                let d = direccion(
                    cara,
                    2.0 * (xx as f64 + 0.5) / self.lado as f64 - 1.0,
                    2.0 * (yy as f64 + 0.5) / self.lado as f64 - 1.0,
                );
                let denominador = dot(n, d);
                let receptor = if denominador < -1e-9 {
                    plano / denominador
                } else {
                    mayor
                };
                let guardado =
                    self.profundidad[cara * self.lado * self.lado + yy * self.lado + xx] as f64;
                let bias = 0.001 + receptor.abs() * 0.0001;
                if receptor <= guardado + bias {
                    visible += wx * wy;
                }
            }
        }
        let intensidad = if !cuarto {
            INTENSIDAD_OBJETOS
        } else if n[1].abs() < 0.5 {
            INTENSIDAD_PAREDES
        } else {
            INTENSIDAD_PISO
        };
        1.0 - intensidad * (1.0 - visible.clamp(0.0, 1.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cubo_sin_obstaculos_y_seis_caras() {
        let mapa = MapaSombras::construir([0.0; 3], 16, &|_| None);
        for cara in 0..6 {
            let p = direccion(cara, 0.3, -0.4).map(|a| a * 4.0);
            let n = p.map(|a| -a / 4.0);
            assert!((mapa.visibilidad(p, n, false) - 1.0).abs() < 1e-9);
        }
    }
    #[test]
    fn distancia_y_pared_mas_tenue() {
        let mapa = MapaSombras::construir([0.0; 3], 32, &|_| Some(2.0));
        let n = [-1.0, 0.0, 0.0];
        assert!((mapa.visibilidad([1.0, 0.0, 0.0], n, false) - 1.0).abs() < 1e-9);
        assert!((mapa.visibilidad([4.0, 0.0, 0.0], n, false) - 0.40).abs() < 1e-9);
        assert!((mapa.visibilidad([4.0, 0.0, 0.0], n, true) - 0.75).abs() < 1e-9);
    }
    #[test]
    fn plano_oblicuo_no_proyecta_sombra_sobre_si_mismo() {
        let n = [-0.8, -0.6, 0.0];
        let mapa = MapaSombras::construir([0.0; 3], 64, &|r| {
            let den = dot(n, r.direction);
            (den < -1e-9).then(|| -4.0 / den)
        });
        for y in -20..20 {
            let y = y as f64 * 0.1;
            let p = [(4.0 - 0.6 * y) / 0.8, y, 0.0];
            assert!((mapa.visibilidad(p, n, false) - 1.0).abs() < 1e-9);
        }
    }
}
