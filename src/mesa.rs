use crate::rayIntersect::{Objeto, Ray};
use crate::textura::{ColorSolido, Fieltro, Textura, TexturaMadera};

const EPSILON: f64 = 0.0001;
const RADIO_TRONERA: f64 = 0.225;
// Altura compartida con bolas, taco y camara; troneras apenas sobre el pano.
pub const ELEVACION_MESA: f64 = 0.90;
pub const ALTURA_PANO: f64 = 0.55 + ELEVACION_MESA;
const ALTURA_TRONERA: f64 = ALTURA_PANO + 0.002;

/// Tiza maciza menos una esfera: la hendidura es geometria, no un disco pintado.
struct Tiza {
    caja: CajaRectangular,
    centro_hueco: [f64; 3],
    radio_hueco: f64,
    techo: f64,
    material: MaterialTiza,
}

impl Tiza {
    fn new(x: f64, z: f64) -> Self {
        let lado = 0.065;
        let base = ELEVACION_MESA + 0.82;
        let techo = base + lado;
        Self {
            caja: CajaRectangular::new([x, base + lado * 0.5, z], [lado; 3], negro()),
            centro_hueco: [x, techo + 0.014, z],
            radio_hueco: 0.024,
            techo,
            material: MaterialTiza,
        }
    }
    fn distancia_hueco2(&self, p: [f64; 3]) -> f64 {
        (0..3).map(|i| (p[i] - self.centro_hueco[i]).powi(2)).sum()
    }
    fn dentro_caja(&self, p: [f64; 3]) -> bool {
        (0..3).all(|i| p[i] >= self.caja.minimo[i] - 1e-9 && p[i] <= self.caja.maximo[i] + 1e-9)
    }
}

impl Objeto for Tiza {
    fn intersect(&self, ray: &Ray) -> Option<f64> {
        let (entrada, salida) = self.caja.intervalo(ray)?;
        if salida <= 1e-7 {
            return None;
        }
        let mut cercano = f64::INFINITY;
        // Si la entrada ya toca material solido, nada dentro puede estar mas cerca.
        if entrada > 1e-7
            && self.distancia_hueco2(ray.point_at(entrada)) >= self.radio_hueco.powi(2)
        {
            return Some(entrada);
        }
        // Caras de la caja que sobreviven a la sustraccion esferica.
        for t in [entrada, salida] {
            if t > 1e-7 && self.distancia_hueco2(ray.point_at(t)) >= self.radio_hueco.powi(2) {
                cercano = cercano.min(t);
            }
        }
        let oc = crate::rayIntersect::sub(ray.origin, self.centro_hueco);
        let b = crate::rayIntersect::dot(oc, ray.direction);
        let c = crate::rayIntersect::dot(oc, oc) - self.radio_hueco.powi(2);
        let disc = b * b - c;
        if disc >= 0.0 {
            let raiz = disc.sqrt();
            for t in [-b - raiz, -b + raiz] {
                if t > 1e-7 && self.dentro_caja(ray.point_at(t)) {
                    cercano = cercano.min(t);
                }
            }
        }
        cercano.is_finite().then_some(cercano)
    }
    fn normal(&self, p: [f64; 3]) -> [f64; 3] {
        if (self.distancia_hueco2(p) - self.radio_hueco.powi(2)).abs() < 1e-9 {
            // Hacia el espacio vacio de la cavidad, no hacia el interior de la tiza.
            crate::rayIntersect::normalize(crate::rayIntersect::sub(self.centro_hueco, p))
        } else {
            self.caja.normal(p)
        }
    }
    fn uv(&self, p: [f64; 3]) -> (f64, f64) {
        // Banda azul superior, incluida toda la cavidad. Cubierta roja debajo.
        (0.0, if p[1] >= self.techo - 0.012 { 1.0 } else { 0.0 })
    }
    fn textura(&self) -> &dyn Textura {
        &self.material
    }
}

struct MaterialTiza;
impl Textura for MaterialTiza {
    fn albedo(&self, _: f64, v: f64) -> crate::textura::Albedo {
        if v > 0.5 {
            [0.04, 0.52, 0.72]
        } else {
            [0.48, 0.015, 0.045]
        }
    }
}

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

/// Limites geometricos para construir el BVH estatico de la mesa.
#[derive(Clone, Copy)]
struct Limites {
    min: [f64; 3],
    max: [f64; 3],
}
impl Limites {
    fn centro_tamano(c: [f64; 3], t: [f64; 3]) -> Self {
        Self {
            min: std::array::from_fn(|i| c[i] - t[i] * 0.5 - 1e-6),
            max: std::array::from_fn(|i| c[i] + t[i] * 0.5 + 1e-6),
        }
    }
    fn unir(self, b: Self) -> Self {
        Self {
            min: std::array::from_fn(|i| self.min[i].min(b.min[i])),
            max: std::array::from_fn(|i| self.max[i].max(b.max[i])),
        }
    }
    fn entrada(&self, ray: &Ray, inv: &[f64; 3], limite: f64) -> Option<f64> {
        let mut cerca = 0.0_f64;
        let mut lejos = limite;
        for i in 0..3 {
            if ray.direction[i] == 0.0 {
                if ray.origin[i] < self.min[i] || ray.origin[i] > self.max[i] {
                    return None;
                }
            } else {
                let a = (self.min[i] - ray.origin[i]) * inv[i];
                let b = (self.max[i] - ray.origin[i]) * inv[i];
                cerca = cerca.max(a.min(b));
                lejos = lejos.min(a.max(b));
                if lejos < cerca {
                    return None;
                }
            }
        }
        Some(cerca)
    }
}
struct Pieza {
    objeto: Box<dyn Objeto>,
    limites: Limites,
}
impl Pieza {
    fn new(objeto: Box<dyn Objeto>, centro: [f64; 3], tamano: [f64; 3]) -> Self {
        Self {
            objeto,
            limites: Limites::centro_tamano(centro, tamano),
        }
    }
}
struct Nodo {
    limites: Limites,
    contenido: Contenido,
}
enum Contenido {
    Hoja(Vec<usize>),
    Rama(Box<Nodo>, Box<Nodo>),
}
impl Nodo {
    fn construir(indices: &mut [usize], piezas: &[Pieza]) -> Self {
        let limites = indices
            .iter()
            .map(|&i| piezas[i].limites)
            .reduce(Limites::unir)
            .unwrap();
        let area = |b: Limites| {
            let d: [f64; 3] = std::array::from_fn(|i| b.max[i] - b.min[i]);
            2.0 * (d[0] * d[1] + d[0] * d[2] + d[1] * d[2])
        };
        let mut costo = indices.len() as f64;
        let mut mejor = None;
        let mut orden = indices.to_vec();
        for eje in 0..3 {
            orden.sort_by(|&a, &b| {
                let a = piezas[a].limites;
                let b = piezas[b].limites;
                (a.min[eje] + a.max[eje]).total_cmp(&(b.min[eje] + b.max[eje]))
            });
            for k in 1..orden.len() {
                let a = orden[..k]
                    .iter()
                    .map(|&i| piezas[i].limites)
                    .reduce(Limites::unir)
                    .unwrap();
                let b = orden[k..]
                    .iter()
                    .map(|&i| piezas[i].limites)
                    .reduce(Limites::unir)
                    .unwrap();
                let c =
                    1.0 + (area(a) * k as f64 + area(b) * (orden.len() - k) as f64) / area(limites);
                if c < costo {
                    costo = c;
                    mejor = Some((orden.clone(), k));
                }
            }
        }
        let contenido = if let Some((orden, k)) = mejor {
            indices.copy_from_slice(&orden);
            let (a, b) = indices.split_at_mut(k);
            Contenido::Rama(
                Box::new(Self::construir(a, piezas)),
                Box::new(Self::construir(b, piezas)),
            )
        } else {
            Contenido::Hoja(indices.to_vec())
        };
        Self { limites, contenido }
    }
    fn ocluye(&self, ray: &Ray, inv: &[f64; 3], piezas: &[Pieza], max: f64) -> bool {
        if self.limites.entrada(ray, inv, max).is_none() {
            return false;
        }
        match &self.contenido {
            Contenido::Hoja(indices) => indices.iter().any(|&i| {
                let obj = piezas[i].objeto.as_ref();
                // Aproximacion economica: los vasos dejan pasar la luz de sombra.
                obj.textura().transparencia() == 0.0
                    && obj.intersect(ray).is_some_and(|t| t > 0.0 && t < max)
            }),
            Contenido::Rama(a, b) => {
                a.ocluye(ray, inv, piezas, max) || b.ocluye(ray, inv, piezas, max)
            }
        }
    }
    fn recorrer(
        &self,
        ray: &Ray,
        inv: &[f64; 3],
        piezas: &[Pieza],
        hit: &mut Option<(usize, f64)>,
    ) {
        match &self.contenido {
            Contenido::Hoja(indices) => {
                for i in indices {
                    if let Some(t) = piezas[*i].objeto.intersect(ray) {
                        if hit
                            .map(|(j, d)| t < d || (t == d && *i < j))
                            .unwrap_or(true)
                        {
                            *hit = Some((*i, t));
                        }
                    }
                }
            }
            Contenido::Rama(a, b) => {
                let max = hit.map(|(_, t)| t).unwrap_or(f64::INFINITY);
                let ta = a.limites.entrada(ray, inv, max);
                let tb = b.limites.entrada(ray, inv, max);
                match (ta, tb) {
                    (Some(ta), Some(tb)) => {
                        let (primero, segundo, tsegundo) =
                            if ta <= tb { (a, b, tb) } else { (b, a, ta) };
                        primero.recorrer(ray, inv, piezas, hit);
                        if tsegundo <= hit.map(|(_, t)| t).unwrap_or(f64::INFINITY) {
                            segundo.recorrer(ray, inv, piezas, hit);
                        }
                    }
                    (Some(_), None) => a.recorrer(ray, inv, piezas, hit),
                    (None, Some(_)) => b.recorrer(ray, inv, piezas, hit),
                    _ => {}
                }
            }
        }
    }
    fn recorrer_opacos(
        &self,
        ray: &Ray,
        inv: &[f64; 3],
        piezas: &[Pieza],
        hit: &mut Option<(usize, f64)>,
    ) {
        match &self.contenido {
            Contenido::Hoja(indices) => {
                for i in indices {
                    if piezas[*i].objeto.textura().transparencia() > 0.0 {
                        continue;
                    }
                    if let Some(t) = piezas[*i].objeto.intersect(ray) {
                        if hit
                            .map(|(j, d)| t < d || (t == d && *i < j))
                            .unwrap_or(true)
                        {
                            *hit = Some((*i, t));
                        }
                    }
                }
            }
            Contenido::Rama(a, b) => {
                let max = hit.map(|(_, t)| t).unwrap_or(f64::INFINITY);
                let ta = a.limites.entrada(ray, inv, max);
                let tb = b.limites.entrada(ray, inv, max);
                match (ta, tb) {
                    (Some(ta), Some(tb)) => {
                        let (primero, segundo, tsegundo) =
                            if ta <= tb { (a, b, tb) } else { (b, a, ta) };
                        primero.recorrer_opacos(ray, inv, piezas, hit);
                        if tsegundo <= hit.map(|(_, t)| t).unwrap_or(f64::INFINITY) {
                            segundo.recorrer_opacos(ray, inv, piezas, hit);
                        }
                    }
                    (Some(_), None) => a.recorrer_opacos(ray, inv, piezas, hit),
                    (None, Some(_)) => b.recorrer_opacos(ray, inv, piezas, hit),
                    _ => {}
                }
            }
        }
    }
}
pub struct Mesa {
    raiz: Nodo,
    piezas: Vec<Pieza>,
}

impl Mesa {
    pub fn ocluye(&self, ray: &Ray, max: f64) -> bool {
        self.raiz
            .ocluye(ray, &ray.direction.map(|d| 1.0 / d), &self.piezas, max)
    }
    pub fn intersectar<'a>(&'a self, ray: &Ray) -> Option<(&'a dyn Objeto, f64)> {
        let inv = ray.direction.map(|d| 1.0 / d);
        self.raiz.limites.entrada(ray, &inv, f64::INFINITY)?;
        let mut cercano = None;
        self.raiz.recorrer(ray, &inv, &self.piezas, &mut cercano);
        cercano.map(|(i, t)| (self.piezas[i].objeto.as_ref(), t))
    }
    pub fn intersectar_opacos<'a>(&'a self, ray: &Ray) -> Option<(&'a dyn Objeto, f64)> {
        let inv = ray.direction.map(|d| 1.0 / d);
        self.raiz.limites.entrada(ray, &inv, f64::INFINITY)?;
        let mut cercano = None;
        self.raiz
            .recorrer_opacos(ray, &inv, &self.piezas, &mut cercano);
        cercano.map(|(i, t)| (self.piezas[i].objeto.as_ref(), t))
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
    piezas: &mut Vec<Pieza>,
    centro: [f64; 3],
    tamano: [f64; 3],
    textura: Box<dyn Textura>,
) {
    piezas.push(Pieza::new(
        Box::new(CajaRectangular::new(centro, tamano, textura)),
        centro,
        tamano,
    ));
}

/// Construye una mesa de 3 x 6 unidades centrada en z = -4.2.
/// El pano queda a ALTURA_PANO; cuatro patas apoyan el cuerpo sobre el piso.
pub fn crear_mesa() -> Mesa {
    let mut piezas: Vec<Pieza> = Vec::new();

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
        piezas.push(Pieza::new(
            Box::new(DiscoHorizontal::new(posicion, RADIO_TRONERA, negro())),
            posicion,
            [RADIO_TRONERA * 2.0, 0.0, RADIO_TRONERA * 2.0],
        ));
    }
    // Cuatro patas torneadas; el extremo superior se inserta en el cuerpo.
    let altura_pata = ELEVACION_MESA + 0.07;
    for x in [-1.35, 1.35] {
        for z in [-6.80, -1.60] {
            piezas.push(Pieza::new(
                Box::new(crate::patas::PataTorneada::new(x, z, altura_pata)),
                [x, altura_pata * 0.5, z],
                [0.5, altura_pata, 0.5],
            ));
        }
    }
    // Cada tiza aporta sus limites exactos al arbol, sin ampliar toda la mesa.
    for (x, z) in [(-1.68, -2.05), (1.68, -6.35)] {
        let tiza = Tiza::new(x, z);
        let centro = std::array::from_fn(|i| (tiza.caja.minimo[i] + tiza.caja.maximo[i]) * 0.5);
        let tamano = std::array::from_fn(|i| tiza.caja.maximo[i] - tiza.caja.minimo[i]);
        piezas.push(Pieza::new(Box::new(tiza), centro, tamano));
    }
    // Limites conservadores para el taco entre sus extremos, con el radio mayor.
    piezas.push(Pieza::new(
        Box::new(crate::taco::crear_taco()),
        [-0.95, ALTURA_PANO + 0.035, -3.975],
        [0.296, 0.124, 4.046],
    ));
    // Segundo taco inclinado por fuera del lateral izquierdo. No cruza la lampara.
    let inicio: [f64; 3] = [-2.5, 0.017, -6.85];
    let fin: [f64; 3] = [-1.128, 3.90, -6.85];
    let radio = 0.048;
    let centro = std::array::from_fn(|i| (inicio[i] + fin[i]) * 0.5);
    let tamano = std::array::from_fn(|i| (inicio[i] - fin[i]).abs() + radio * 2.0);
    piezas.push(Pieza::new(
        Box::new(crate::taco::Taco::new(inicio, fin, radio, 0.020)),
        centro,
        tamano,
    ));

    // Ambos vasos en el riel lateral DERECHO: ancho 0.36, diametro del vaso 0.22.
    // Separados de la tronera delantera y de la tiza trasera.
    for z in [-1.85, -2.20] {
        let base = [1.68, ELEVACION_MESA + 0.82, z];
        let vaso = crate::vasos::Vaso::new(base);
        let (centro, tamano) = vaso.limites();
        piezas.push(Pieza::new(Box::new(vaso), centro, tamano));
    }
    let mut indices: Vec<usize> = (0..piezas.len()).collect();
    let raiz = Nodo::construir(&mut indices, &piezas);
    Mesa { raiz, piezas }
}
#[cfg(test)]
mod tests {
    #[test]
    fn profundidad_opaca_ignora_vasos_y_coincide_con_lineal() {
        let mesa = super::crear_mesa();
        let mut seed = 4321u64;
        let mut rnd = || {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            (seed >> 11) as f64 / ((1u64 << 53) as f64)
        };
        for _ in 0..10000 {
            let ray = super::Ray::new(
                crate::lampara::LUZ_POSICION,
                [rnd() - 0.5, rnd() - 0.5, rnd() - 0.5],
            );
            let lineal = mesa
                .piezas
                .iter()
                .filter(|p| p.objeto.textura().transparencia() == 0.0)
                .filter_map(|p| p.objeto.intersect(&ray))
                .min_by(f64::total_cmp);
            assert_eq!(mesa.intersectar_opacos(&ray).map(|(_, t)| t), lineal);
        }
    }
    #[test]
    fn oclusion_bvh_coincide_con_lineal_y_respeta_distancia() {
        let mesa = super::crear_mesa();
        let mut seed = 1234567u64;
        let mut rnd = || {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            (seed >> 11) as f64 / ((1u64 << 53) as f64)
        };
        for _ in 0..10000 {
            let ray = super::Ray::new(
                [rnd() * 10.0 - 5.0, rnd() * 7.0, rnd() * 16.0 - 12.0],
                [rnd() - 0.5, rnd() - 0.5, rnd() - 0.5],
            );
            let max = rnd() * 12.0;
            let lineal = mesa.piezas.iter().any(|p| {
                p.objeto.textura().transparencia() == 0.0
                    && p.objeto.intersect(&ray).is_some_and(|t| t > 0.0 && t < max)
            });
            assert_eq!(mesa.ocluye(&ray, max), lineal);
            assert!(!mesa.ocluye(&ray, 0.0));
        }
    }

    use super::*;
    #[test]
    fn vasos_sobre_madera_y_taco_fuera_de_lampara() {
        let mesa = crear_mesa();
        for z in [-1.85, -2.20] {
            assert!(crate::vasos::RADIO < 0.36 * 0.5);
            let ray = Ray::new([1.68, 3.0, z], [0.0, -1.0, 0.0]);
            let (obj, t) = mesa.intersectar(&ray).unwrap();
            assert!(
                (ray.point_at(t)[1] - (ELEVACION_MESA + 0.82 + crate::vasos::ALTO)).abs() < 1e-8
            );
            assert!(obj.textura().transparencia() > 0.0);
        }
        let lampara = crate::lampara::crear_lampara();
        for i in 0..101 {
            let y = 3.55 + i as f64 * 0.004;
            assert!(
                lampara
                    .intersectar(&Ray::new([-3.0, y, -6.85], [1.0, 0.0, 0.0]))
                    .is_none()
            );
        }
    }
    #[test]
    fn bvh_coincide_con_recorrido_lineal() {
        let mesa = crear_mesa();
        let mut estado = 12345_u64;
        let mut aleatorio = || {
            estado = estado.wrapping_mul(6364136223846793005).wrapping_add(1);
            (estado >> 32) as f64 / u32::MAX as f64
        };
        for _ in 0..20000 {
            let origen = [
                aleatorio() * 8.0 - 4.0,
                aleatorio() * 4.0,
                aleatorio() * 12.0 - 10.0,
            ];
            let direccion = [
                aleatorio() * 2.0 - 1.0,
                aleatorio() * 2.0 - 1.0,
                aleatorio() * 2.0 - 1.0,
            ];
            let ray = Ray::new(origen, direccion);
            let lineal = mesa
                .piezas
                .iter()
                .filter_map(|p| p.objeto.intersect(&ray))
                .min_by(f64::total_cmp);
            let acelerado = mesa.intersectar(&ray).map(|(_, t)| t);
            match (lineal, acelerado) {
                (None, None) => {}
                (Some(a), Some(b)) => assert!((a - b).abs() < 1e-8),
                _ => panic!("descarte incorrecto: {:?} {:?}", lineal, acelerado),
            }
        }
    }
    #[test]
    fn tiza_concava_y_envolvente() {
        let tiza = Tiza::new(-1.68, -2.05);
        let ray = Ray::new([-1.68, tiza.techo + 0.1, -2.05], [0.0, -1.0, 0.0]);
        let t = tiza.intersect(&ray).unwrap();
        assert!((t - 0.110).abs() < 1e-8);
        assert!(tiza.normal(ray.point_at(t))[1] > 0.999);
        let borde = Ray::new([-1.68 + 0.029, tiza.techo + 0.1, -2.05], [0.0, -1.0, 0.0]);
        assert!((tiza.intersect(&borde).unwrap() - 0.1).abs() < 1e-8);
        let mesa = crear_mesa();
        assert!((mesa.intersectar(&ray).unwrap().1 - t).abs() < 1e-8);
        let lateral = Ray::new([-1.5, tiza.techo - 0.03, -2.05], [-1.0, 0.0, 0.0]);
        let t = tiza.intersect(&lateral).unwrap();
        assert_eq!(tiza.uv(lateral.point_at(t)).1, 0.0);
    }
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
