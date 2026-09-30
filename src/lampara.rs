use crate::cuarto::Y_MAX;
use crate::mesa::CajaRectangular;
use crate::rayIntersect::{Objeto, Ray, cross, dot, escala, normalize, sub, suma};
use crate::textura::{Albedo, ColorSolido, Textura};

pub const LUZ_POSICION: [f64; 3] = [0.0, 3.55, -4.20];
// Proporcion del ancho y largo del pano (3 x 6 unidades).
// Usa 0.60 para una lampara mas pequena; 0.75 equivale a tres cuartas partes.
pub const PROPORCION_MESA: f64 = 0.75;
const ESCALA_X: f64 = 3.0 * PROPORCION_MESA / 2.17;
const ESCALA_Z: f64 = 6.0 * PROPORCION_MESA / 4.88;
fn escalar_posicion(p: [f64; 3]) -> [f64; 3] {
    [p[0] * ESCALA_X, p[1], -4.20 + (p[2] + 4.20) * ESCALA_Z]
}
const BASE: f64 = 3.65;
const CIMA: f64 = 4.35;

/// Trapecio plano con bases paralelas. UV continuo para pintar las divisiones.
struct Pantalla {
    a: [f64; 3],
    b: [f64; 3],
    c: [f64; 3],
    d: [f64; 3],
    normal: [f64; 3],
    vertical: [f64; 3],
    altura2: f64,
    textura: Vitral,
}

impl Pantalla {
    fn new(
        a: [f64; 3],
        b: [f64; 3],
        c: [f64; 3],
        d: [f64; 3],
        divisiones: f64,
        difusor: bool,
    ) -> Self {
        let horizontal = normalize(sub(b, a));
        let ad = sub(d, a);
        let vertical = sub(ad, escala(horizontal, dot(ad, horizontal)));
        Self {
            a,
            b,
            c,
            d,
            normal: normalize(cross(sub(b, a), ad)),
            vertical,
            altura2: dot(vertical, vertical),
            textura: Vitral {
                divisiones,
                difusor,
            },
        }
    }
    fn coordenadas(&self, p: [f64; 3]) -> (f64, f64) {
        let v = dot(sub(p, self.a), self.vertical) / self.altura2;
        let izquierda = suma(self.a, escala(sub(self.d, self.a), v));
        let derecha = suma(self.b, escala(sub(self.c, self.b), v));
        let ancho = sub(derecha, izquierda);
        (dot(sub(p, izquierda), ancho) / dot(ancho, ancho), v)
    }
}

impl Objeto for Pantalla {
    fn intersect(&self, ray: &Ray) -> Option<f64> {
        let denom = dot(ray.direction, self.normal);
        if denom.abs() < 1e-10 {
            return None;
        }
        let t = dot(sub(self.a, ray.origin), self.normal) / denom;
        if t <= 1e-7 {
            return None;
        }
        let (u, v) = self.coordenadas(ray.point_at(t));
        ((-1e-8..=1.0 + 1e-8).contains(&u) && (-1e-8..=1.0 + 1e-8).contains(&v)).then_some(t)
    }
    fn normal(&self, _: [f64; 3]) -> [f64; 3] {
        self.normal
    }
    fn uv(&self, p: [f64; 3]) -> (f64, f64) {
        self.coordenadas(p)
    }
    fn textura(&self) -> &dyn Textura {
        &self.textura
    }
}

struct Vitral {
    divisiones: f64,
    difusor: bool,
}
impl Textura for Vitral {
    fn albedo(&self, u: f64, v: f64) -> Albedo {
        let borde = [0.035, 0.025, 0.018];
        if !(0.035..=0.965).contains(&u) || !(0.06..=0.94).contains(&v) {
            return borde;
        }
        let celda = (u * self.divisiones).rem_euclid(1.0);
        if celda < 0.035 || celda > 0.965 {
            return borde;
        }
        if self.difusor {
            if (v * 3.0).rem_euclid(1.0) < 0.025 {
                return borde;
            }
            return [1.0, 0.77, 0.30];
        }
        if v < 0.22 || v > 0.80 {
            let fila = if v < 0.22 {
                (v - 0.06) / 0.16
            } else {
                (v - 0.80) / 0.14
            };
            let rombo = (celda - 0.5).abs() * 2.0 + (fila - 0.5).abs() * 2.0;
            if (rombo - 0.80).abs() < 0.14 {
                return borde;
            }
            return [0.65, 0.23, 0.045];
        }
        if (v - 0.23).abs() < 0.018 || (v - 0.79).abs() < 0.018 {
            return borde;
        }
        let variacion = 0.90 + 0.10 * (u * self.divisiones).floor().rem_euclid(3.0) / 2.0;
        [variacion, 0.61 + 0.12 * v, 0.09]
    }
    fn emision(&self) -> f64 {
        1.0
    }
}

pub struct Lampara {
    limite: CajaRectangular,
    piezas: Vec<Box<dyn Objeto>>,
}
impl Lampara {
    pub fn intersectar(&self, ray: &Ray) -> Option<(&dyn Objeto, f64)> {
        self.limite.intersect(ray)?;
        let mut cercano: Option<(&dyn Objeto, f64)> = None;
        for pieza in &self.piezas {
            if let Some(t) = pieza.intersect(ray) {
                if cercano.map(|(_, d)| t < d).unwrap_or(true) {
                    cercano = Some((pieza.as_ref(), t));
                }
            }
        }
        cercano
    }
}

pub fn crear_lampara() -> Lampara {
    assert!(Y_MAX > CIMA + 0.15, "el techo debe quedar sobre la lampara");
    assert!(PROPORCION_MESA > 0.0);
    let caja = |centro: [f64; 3], tamano: [f64; 3], material: Box<dyn Textura>| {
        CajaRectangular::new(
            escalar_posicion(centro),
            [tamano[0] * ESCALA_X, tamano[1], tamano[2] * ESCALA_Z],
            material,
        )
    };
    let metal = || Box::new(ColorSolido::new([0.07, 0.055, 0.04])) as Box<dyn Textura>;
    let mut piezas: Vec<Box<dyn Objeto>> = Vec::new();
    // Dos varillas y rosetas de montaje conectadas al techo configurado.
    for z in [-5.45, -2.95] {
        piezas.push(Box::new(caja(
            [0.0, (CIMA + Y_MAX) * 0.5, z],
            [0.045, Y_MAX - CIMA, 0.045],
            metal(),
        )));
        piezas.push(Box::new(caja(
            [0.0, Y_MAX - 0.035, z],
            [0.28, 0.07, 0.28],
            metal(),
        )));
    }
    // Esquinas inferiores y superiores, alrededor de z=-4.2.
    let inferior = [
        [-1.05, BASE, -6.60],
        [1.05, BASE, -6.60],
        [1.05, BASE, -1.80],
        [-1.05, BASE, -1.80],
    ];
    let superior = [
        [-0.60, CIMA, -6.00],
        [0.60, CIMA, -6.00],
        [0.60, CIMA, -2.40],
        [-0.60, CIMA, -2.40],
    ];
    for i in 0..4 {
        let j = (i + 1) % 4;
        piezas.push(Box::new(Pantalla::new(
            escalar_posicion(inferior[i]),
            escalar_posicion(inferior[j]),
            escalar_posicion(superior[j]),
            escalar_posicion(superior[i]),
            if i % 2 == 0 { 4.0 } else { 10.0 },
            false,
        )));
    }
    piezas.push(Box::new(Pantalla::new(
        escalar_posicion(inferior[0]),
        escalar_posicion(inferior[1]),
        escalar_posicion(inferior[2]),
        escalar_posicion(inferior[3]),
        4.0,
        true,
    )));
    piezas.push(Box::new(caja(
        [0.0, CIMA + 0.025, -4.20],
        [1.28, 0.05, 3.68],
        metal(),
    )));
    // Marco inferior con grosor real; el resto de la traceria es procedural.
    for x in [-1.05, 1.05] {
        piezas.push(Box::new(caja(
            [x, BASE, -4.20],
            [0.07, 0.08, 4.88],
            metal(),
        )));
    }
    for z in [-6.60, -1.80] {
        piezas.push(Box::new(caja([0.0, BASE, z], [2.17, 0.08, 0.07], metal())));
    }
    Lampara {
        limite: caja(
            [0.0, (BASE - 0.05 + Y_MAX) * 0.5, -4.20],
            [2.20, Y_MAX - BASE + 0.05, 4.90],
            metal(),
        ),
        piezas,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pantalla_trapezoidal_y_difusor() {
        let p = Pantalla::new(
            [-2.0, 0.0, 0.0],
            [2.0, 0.0, 0.0],
            [1.0, 2.0, 0.0],
            [-1.0, 2.0, 0.0],
            4.0,
            false,
        );
        assert_eq!(
            p.intersect(&Ray::new([0.0, 1.0, 1.0], [0.0, 0.0, -1.0])),
            Some(1.0)
        );
        assert!(
            p.intersect(&Ray::new([1.8, 1.8, 1.0], [0.0, 0.0, -1.0]))
                .is_none()
        );
        let l = crear_lampara();
        let (obj, t) = l
            .intersectar(&Ray::new([0.0, 2.0, -4.2], [0.0, 1.0, 0.0]))
            .unwrap();
        assert!((t - (BASE - 2.0)).abs() < 1e-8);
        assert_eq!(obj.textura().emision(), 1.0);
        assert!(LUZ_POSICION[1] < BASE);
    }
}
