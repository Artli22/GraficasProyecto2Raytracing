use crate::rayIntersect::{Objeto, Ray, dot, normalize, sub};
use crate::textura::{Textura, uv_esfera};

pub struct Sphere {
    pub center: [f64; 3],
    pub radius: f64,
    pub textura: Box<dyn Textura>,
}

impl Sphere {
    pub fn new(center: [f64; 3], radius: f64, textura: Box<dyn Textura>) -> Self {
        Sphere { center, radius, textura }
    }
}

impl Objeto for Sphere {
    fn intersect(&self, ray: &Ray) -> Option<f64> {
        let oc = sub(ray.origin, self.center);
        let b = 2.0 * dot(oc, ray.direction);
        let c = dot(oc, oc) - self.radius * self.radius;
        let discriminant = b * b - 4.0 * c;
        if discriminant < 0.0 {
            return None;
        }
        let t = (-b - discriminant.sqrt()) / 2.0;
        if t > 0.0 { Some(t) } else { None }
    }

    fn normal(&self, punto: [f64; 3]) -> [f64; 3] {
        normalize(sub(punto, self.center))
    }

    fn uv(&self, punto: [f64; 3]) -> (f64, f64) {
        uv_esfera(sub(punto, self.center), self.radius)
    }

    fn textura(&self) -> &dyn Textura {
        self.textura.as_ref()
    }
}
