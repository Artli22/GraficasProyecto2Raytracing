use crate::rayIntersect::{Objeto, Ray};
use crate::textura::{ColorSolido, Fieltro, Textura, TexturaMadera};

const EPSILON: f64 = 0.0001;
const RADIO_TRONERA: f64 = 0.225;
// Altura compartida con bolas, taco y camara; troneras apenas sobre el pano.
pub const ELEVACION_MESA: f64 = 0.90;
pub const ALTURA_PANO: f64 = 0.55 + ELEVACION_MESA;
const ALTURA_TRONERA: f64 = ALTURA_PANO + 0.002;

/// Circulo horizontal sin volumen, utilizado para representar una tronera
/// completamente plana sobre la superficie de la mesa.
pub struct DiscoHorizontal {
    centro: [f64; 3],
    radio: f64,
    textura: Box<dyn Textura>,
}

impl DiscoHorizontal {
    pub fn new(centro: [f64; 3], radio: f64, textura: Box<dyn Textura>) -> Self {
        DiscoHorizontal {
            centro,
            radio,
            textura,
        }
    }
}

impl Objeto for DiscoHorizontal {
    fn intersect(&self, ray: &Ray) -> Option<f64> {
        if ray.direction[1].abs() < EPSILON {
            return None;
        }

        let t = (self.centro[1] - ray.origin[1]) / ray.direction[1];
        if t <= EPSILON {
            return None;
        }

        let punto = ray.point_at(t);
        let dx = punto[0] - self.centro[0];
        let dz = punto[2] - self.centro[2];
        if dx * dx + dz * dz <= self.radio * self.radio {
            Some(t)
        } else {
            None
        }
    }

    fn normal(&self, _punto: [f64; 3]) -> [f64; 3] {
        [0.0, 1.0, 0.0]
    }

    fn uv(&self, punto: [f64; 3]) -> (f64, f64) {
        (
            0.5 + (punto[0] - self.centro[0]) / (2.0 * self.radio),
            0.5 + (punto[2] - self.centro[2]) / (2.0 * self.radio),
        )
    }

    fn textura(&self) -> &dyn Textura {
        self.textura.as_ref()
    }
}

/// Prisma rectangular alineado con los ejes. Permite construir el tablero,
/// el cuerpo y los rieles con una sola primitiva economica.
pub struct CajaRectangular {
    minimo: [f64; 3],
    maximo: [f64; 3],
    textura: Box<dyn Textura>,
}

impl CajaRectangular {
    pub fn new(centro: [f64; 3], tamano: [f64; 3], textura: Box<dyn Textura>) -> Self {
        let mitad = [tamano[0] * 0.5, tamano[1] * 0.5, tamano[2] * 0.5];
        CajaRectangular {
            minimo: [
                centro[0] - mitad[0],
                centro[1] - mitad[1],
                centro[2] - mitad[2],
            ],
            maximo: [
                centro[0] + mitad[0],
                centro[1] + mitad[1],
                centro[2] + mitad[2],
            ],
            textura,
        }
    }

    fn intervalo(&self, ray: &Ray) -> Option<(f64, f64)> {
        let mut entrada = f64::NEG_INFINITY;
        let mut salida = f64::INFINITY;

        for eje in 0..3 {
            if ray.direction[eje].abs() < EPSILON {
                if ray.origin[eje] < self.minimo[eje] || ray.origin[eje] > self.maximo[eje] {
                    return None;
                }
                continue;
            }

            let inversa = 1.0 / ray.direction[eje];
            let mut t0 = (self.minimo[eje] - ray.origin[eje]) * inversa;
            let mut t1 = (self.maximo[eje] - ray.origin[eje]) * inversa;
            if t0 > t1 {
                std::mem::swap(&mut t0, &mut t1);
            }
            entrada = entrada.max(t0);
            salida = salida.min(t1);
            if salida < entrada {
                return None;
            }
        }

        Some((entrada, salida))
    }
}

impl Objeto for CajaRectangular {
    fn intersect(&self, ray: &Ray) -> Option<f64> {
        let (entrada, salida) = self.intervalo(ray)?;
        if entrada > EPSILON {
            Some(entrada)
        } else if salida > EPSILON {
            Some(salida)
        } else {
            None
        }
    }

    fn normal(&self, punto: [f64; 3]) -> [f64; 3] {
        let distancias = [
            ((punto[0] - self.minimo[0]).abs(), [-1.0, 0.0, 0.0]),
            ((punto[0] - self.maximo[0]).abs(), [1.0, 0.0, 0.0]),
            ((punto[1] - self.minimo[1]).abs(), [0.0, -1.0, 0.0]),
            ((punto[1] - self.maximo[1]).abs(), [0.0, 1.0, 0.0]),
            ((punto[2] - self.minimo[2]).abs(), [0.0, 0.0, -1.0]),
            ((punto[2] - self.maximo[2]).abs(), [0.0, 0.0, 1.0]),
        ];

        distancias
            .iter()
            .min_by(|a, b| a.0.partial_cmp(&b.0).unwrap())
            .map(|(_, normal)| *normal)
            .unwrap()
    }

    fn uv(&self, punto: [f64; 3]) -> (f64, f64) {
        let ancho = self.maximo[0] - self.minimo[0];
        let largo = self.maximo[2] - self.minimo[2];
        (
            (punto[0] - self.minimo[0]) / ancho,
            (punto[2] - self.minimo[2]) / largo,
        )
    }

    fn textura(&self) -> &dyn Textura {
        self.textura.as_ref()
    }
}

/// Agrupa toda la mesa y prueba primero una caja envolvente para no recorrer
/// sus piezas cuando el rayo apunta al cielo o fuera de la mesa.
pub struct Mesa {
    limite: CajaRectangular,
    piezas: Vec<Box<dyn Objeto>>,
}

impl Mesa {
    pub fn intersectar<'a>(&'a self, ray: &Ray) -> Option<(&'a dyn Objeto, f64)> {
        self.limite.intersect(ray)?;
        let mut cercano: Option<(&dyn Objeto, f64)> = None;

        for pieza in &self.piezas {
            if let Some(t) = pieza.intersect(ray) {
                if cercano.map(|(_, distancia)| t < distancia).unwrap_or(true) {
                    cercano = Some((pieza.as_ref(), t));
                }
            }
        }

        cercano
    }
}

fn madera() -> Box<dyn Textura> {
    Box::new(TexturaMadera::new(
        [0.30, 0.085, 0.025],
        [0.14, 0.038, 0.010],
        6.0,
        90.0,
        24.0,
    ))
}

fn pano_verde() -> Box<dyn Textura> {
    Box::new(Fieltro::new([0.025, 0.36, 0.10], 55.0, 0.35))
}

fn negro() -> Box<dyn Textura> {
    Box::new(ColorSolido::new([0.006, 0.006, 0.006]))
}

fn agregar_caja(
    piezas: &mut Vec<Box<dyn Objeto>>,
    centro: [f64; 3],
    tamano: [f64; 3],
    textura: Box<dyn Textura>,
) {
    piezas.push(Box::new(CajaRectangular::new(centro, tamano, textura)));
}

/// Construye una mesa de 3 x 6 unidades centrada en z = -4.2.
/// El pano queda a ALTURA_PANO; cuatro patas apoyan el cuerpo sobre el piso.
pub fn crear_mesa() -> Mesa {
    let mut piezas: Vec<Box<dyn Objeto>> = Vec::new();

    // Cuerpo, faldones y pano.
    agregar_caja(
        &mut piezas,
        [0.0, 0.28 + ELEVACION_MESA, -4.20],
        [3.65, 0.46, 6.65],
        madera(),
    );
    agregar_caja(
        &mut piezas,
        [0.0, ALTURA_PANO - 0.035, -4.20],
        [3.00, 0.07, 6.00],
        pano_verde(),
    );

    // Cuatro rieles continuos de madera. Los laterales pasan por detras de las
    // troneras centrales para que nunca se vea un hueco abierto en la pared.
    for &x in &[-1.68, 1.68] {
        agregar_caja(
            &mut piezas,
            [x, 0.68 + ELEVACION_MESA, -4.20],
            [0.36, 0.28, 6.52],
            madera(),
        );
    }
    for &z in &[-7.28, -1.12] {
        agregar_caja(
            &mut piezas,
            [0.0, 0.68 + ELEVACION_MESA, z],
            [3.05, 0.28, 0.36],
            madera(),
        );
    }

    // Seis troneras negras planas, colocadas apenas por encima de la tela.
    let posiciones_troneras = [
        [-1.48, ALTURA_TRONERA, -7.08],
        [1.48, ALTURA_TRONERA, -7.08],
        [-1.55, ALTURA_TRONERA, -4.20],
        [1.55, ALTURA_TRONERA, -4.20],
        [-1.48, ALTURA_TRONERA, -1.32],
        [1.48, ALTURA_TRONERA, -1.32],
    ];
    for posicion in posiciones_troneras {
        piezas.push(Box::new(DiscoHorizontal::new(
            posicion,
            RADIO_TRONERA,
            negro(),
        )));
    }

    // Cuatro patas torneadas; el extremo superior se inserta en el cuerpo.
    let altura_pata = ELEVACION_MESA + 0.07;
    for x in [-1.35, 1.35] {
        for z in [-6.80, -1.60] {
            piezas.push(Box::new(crate::patas::PataTorneada::new(x, z, altura_pata)));
        }
    }
    // La envolvente incluye las patas y el taco elevado.
    piezas.push(Box::new(crate::taco::crear_taco()));

    Mesa {
        limite: CajaRectangular::new(
            [0.0, (0.84 + ELEVACION_MESA) * 0.5, -4.20],
            [3.90, 0.84 + ELEVACION_MESA, 6.90],
            negro(),
        ),
        piezas,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn patas_visibles_y_espacio_libre_bajo_mesa() {
        let mesa = crear_mesa();
        for x in [-1.35_f64, 1.35] {
            for z in [-6.80, -1.60] {
                let signo = x.signum();
                let ray = Ray::new([signo * 2.5, 0.4, z], [-signo, 0.0, 0.0]);
                let (_, t) = mesa
                    .intersectar(&ray)
                    .expect("la envolvente debe incluir cada pata");
                assert!((0.90..1.10).contains(&t));
            }
        }
        assert!(
            mesa.intersectar(&Ray::new([0.0, 0.4, 0.0], [0.0, 0.0, -1.0]))
                .is_none()
        );
        for bola in crate::billar::crear_bolas() {
            assert!((bola.center[1] - bola.radius - ALTURA_PANO).abs() < 1e-8);
        }
    }
}

