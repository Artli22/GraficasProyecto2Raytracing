use crate::rayIntersect::{Objeto, Ray, dot};
use crate::textura::{aplicar_emision, sombrear_puntual_visible};
use raylib::prelude::Color;

/// Limite de objetos transparentes; cada vaso resuelve su refraccion internamente.
pub const MAX_CAPAS: usize = 6;
pub fn trazar<'a, F>(primario: &Ray, escena: &F) -> Color
where
    F: Fn(&Ray) -> Option<(&'a dyn Objeto, f64, bool)>,
{
    trazar_con_sombras(primario, escena, &|_, _| false)
}

pub fn trazar_con_sombras<'a, F, S>(primario: &Ray, escena: &F, ocluye: &S) -> Color
where
    F: Fn(&Ray) -> Option<(&'a dyn Objeto, f64, bool)>,
    S: Fn(&Ray, f64) -> bool,
{
    trazar_precalculado(primario, escena, &|p, n, _| visibilidad(p, n, ocluye))
}

pub fn trazar_precalculado<'a, F, S>(primario: &Ray, escena: &F, sombra: &S) -> Color
where
    F: Fn(&Ray) -> Option<(&'a dyn Objeto, f64, bool)>,
    S: Fn([f64; 3], [f64; 3], bool) -> f64,
{
    let mut ray = Ray {
        origin: primario.origin,
        direction: primario.direction,
    };
    let mut acumulado = [0.0; 3];
    let mut peso = [1.0; 3];
    for capa in 0..MAX_CAPAS {
        let Some((objeto, t, es_cuarto)) = escena(&ray) else {
            break;
        };
        let mat = objeto.textura();
        let transparencia = mat.transparencia().clamp(0.0, 1.0);
        if transparencia > 0.0 {
            if let Some(paso) = objeto.atravesar(&ray, t) {
                for i in 0..3 {
                    acumulado[i] += peso[i] * paso.aporte[i] * 255.0;
                    peso[i] *= paso.filtro[i];
                }
                ray = paso.rayo;
                if peso.iter().all(|&p| p < 0.01) {
                    break;
                }
                continue;
            }
        }
        let p = ray.point_at(t);
        let n = objeto.normal(p);
        let (u, v) = objeto.uv(p);

        let albedo = mat.albedo(u, v);
        let color = sombrear_puntual_visible(
            ray.direction,
            p,
            n,
            crate::lampara::LUZ_POSICION,
            if es_cuarto {
                [1.0, 0.96, 0.90]
            } else {
                [1.0, 0.72, 0.30]
            },
            albedo,
            mat.brillo(),
            sombra(p, n, es_cuarto),
        );
        let color = if es_cuarto {
            let canal =
                |c: u8, i: usize| (c as f64 + albedo[i] * 0.24 * 255.0).clamp(0.0, 255.0) as u8;
            Color::new(canal(color.r, 0), canal(color.g, 1), canal(color.b, 2), 255)
        } else {
            color
        };
        let color = aplicar_emision(color, albedo, mat.emision());

        // Los objetos opacos terminan tras resolver su iluminacion.
        if capa == 0 && transparencia == 0.0 {
            return color;
        }
        let borde = (1.0 - dot(n, ray.direction).abs().clamp(0.0, 1.0)).powi(5);
        let alpha = if capa + 1 == MAX_CAPAS {
            1.0
        } else {
            1.0 - transparencia * (1.0 - 0.75 * borde)
        };
        let canales = [color.r as f64, color.g as f64, color.b as f64];
        let filtro = mat.filtro();
        for i in 0..3 {
            // Relleno leve del material transparente para un contorno legible.
            let superficie = if transparencia > 0.0 {
                (canales[i] + albedo[i] * 90.0).min(255.0)
            } else {
                canales[i]
            };
            acumulado[i] += peso[i] * alpha * superficie;
            peso[i] *= (1.0 - alpha) * filtro[i];
        }
        if transparencia == 0.0 || peso.iter().all(|&p| p < 0.01) {
            break;
        }
        ray.origin = ray.point_at(t + 1e-5);
    }
    Color::new(
        acumulado[0].clamp(0.0, 255.0) as u8,
        acumulado[1].clamp(0.0, 255.0) as u8,
        acumulado[2].clamp(0.0, 255.0) as u8,
        255,
    )
}

/// Un rayo acotado a la distancia a la luz, desplazado fuera de la superficie.
fn visibilidad<S: Fn(&Ray, f64) -> bool>(p: [f64; 3], n: [f64; 3], ocluye: &S) -> f64 {
    use crate::rayIntersect::{escala, sub, suma};
    let luz = crate::lampara::LUZ_POSICION;
    if dot(n, sub(luz, p)) <= 0.0 {
        return 0.0;
    }
    let origen = suma(p, escala(n, 0.0002));
    let delta = sub(luz, origen);
    let distancia = dot(delta, delta).sqrt();
    if distancia <= 0.0002 {
        return 1.0;
    }
    let rayo = Ray {
        origin: origen,
        direction: escala(delta, 1.0 / distancia),
    };
    if ocluye(&rayo, distancia - 0.0002) {
        0.0
    } else {
        1.0
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn sombra_conserva_ambiente_y_segmento_acotado() {
        let p = [0.0, 1.0, -4.2];
        let n = [0.0, 1.0, 0.0];
        assert_eq!(
            super::visibilidad(p, n, &|r, max| {
                assert!(r.origin[1] > p[1]);
                assert!(max > 2.5 && max < 2.55);
                true
            }),
            0.0
        );
        assert_eq!(
            super::visibilidad(p, [0.0, -1.0, 0.0], &|_, _| panic!("cara opuesta")),
            0.0
        );
        let c = crate::textura::sombrear_puntual_visible(
            [0.0, -1.0, 0.0],
            p,
            n,
            crate::lampara::LUZ_POSICION,
            [1.0; 3],
            [1.0; 3],
            32.0,
            0.0,
        );
        assert_eq!([c.r, c.g, c.b], [19; 3]);
    }

    use super::*;
    use crate::textura::{Albedo, Textura};
    struct Transparente;
    impl Textura for Transparente {
        fn albedo(&self, _: f64, _: f64) -> Albedo {
            [1.0; 3]
        }
        fn transparencia(&self) -> f64 {
            1.0
        }
    }
    struct Superficie {
        material: Transparente,
    }
    impl Objeto for Superficie {
        fn intersect(&self, _: &Ray) -> Option<f64> {
            Some(1.0)
        }
        fn normal(&self, _: [f64; 3]) -> [f64; 3] {
            [0.0, 0.0, 1.0]
        }
        fn uv(&self, _: [f64; 3]) -> (f64, f64) {
            (0.0, 0.0)
        }
        fn textura(&self) -> &dyn Textura {
            &self.material
        }
    }
    #[test]
    fn transmision_tiene_limite_y_escena_vacia_es_negra() {
        let obj = Superficie {
            material: Transparente,
        };
        let consultas = std::cell::Cell::new(0);
        let ray = Ray::new([0.0; 3], [0.0, 0.0, -1.0]);
        let _ = trazar(&ray, &|_| {
            consultas.set(consultas.get() + 1);
            Some((&obj as &dyn Objeto, 1.0, false))
        });
        assert_eq!(consultas.get(), MAX_CAPAS);
        let negro = trazar(&ray, &|_| None);
        assert_eq!([negro.r, negro.g, negro.b], [0; 3]);
    }
}
