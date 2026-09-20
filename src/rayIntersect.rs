use crate::textura::Textura;

pub struct Ray {
    pub origin: [f64; 3],
    pub direction: [f64; 3],
}

impl Ray {
    pub fn new(origin: [f64; 3], direction: [f64; 3]) -> Self {
        Ray {
            origin,
            direction: normalize(direction),
        }
    }

    pub fn point_at(&self, t: f64) -> [f64; 3] {
        [
            self.origin[0] + t * self.direction[0],
            self.origin[1] + t * self.direction[1],
            self.origin[2] + t * self.direction[2],
        ]
    }
}

/// Cualquier cuerpo geometrico que un rayo puede golpear (esfera, cubo, etc.).
pub trait Objeto {
    /// Distancia t al impacto mas cercano, o None si el rayo no lo toca.
    fn intersect(&self, ray: &Ray) -> Option<f64>;
    fn normal(&self, punto: [f64; 3]) -> [f64; 3];
    fn uv(&self, punto: [f64; 3]) -> (f64, f64);
    fn textura(&self) -> &dyn Textura;
}

pub fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

pub fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

pub fn suma(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

pub fn escala(v: [f64; 3], k: f64) -> [f64; 3] {
    [v[0] * k, v[1] * k, v[2] * k]
}

pub fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

pub fn normalize(v: [f64; 3]) -> [f64; 3] {
    let len = dot(v, v).sqrt();
    [v[0] / len, v[1] / len, v[2] / len]
}
