use crate::rayIntersect::{Objeto, Ray};
use crate::textura::Textura;

/// Cubo alineado a los ejes (AABB), definido por sus esquinas min/max.
pub struct Cubo {
    pub min: [f64; 3],
    pub max: [f64; 3],
    pub textura: Box<dyn Textura>,
}

impl Cubo {
    pub fn new(centro: [f64; 3], medio_lado: f64, textura: Box<dyn Textura>) -> Self {
        Cubo {
            min: [centro[0] - medio_lado, centro[1] - medio_lado, centro[2] - medio_lado],
            max: [centro[0] + medio_lado, centro[1] + medio_lado, centro[2] + medio_lado],
            textura,
        }
    }
}

impl Objeto for Cubo {
    fn intersect(&self, ray: &Ray) -> Option<f64> {
        // Interseccion rayo-AABB por el metodo de "slabs".
        let mut t_min = f64::NEG_INFINITY;
        let mut t_max = f64::INFINITY;
        for i in 0..3 {
            if ray.direction[i].abs() < 1e-12 {
                if ray.origin[i] < self.min[i] || ray.origin[i] > self.max[i] {
                    return None;
                }
                continue;
            }
            let inv_d = 1.0 / ray.direction[i];
            let mut t1 = (self.min[i] - ray.origin[i]) * inv_d;
            let mut t2 = (self.max[i] - ray.origin[i]) * inv_d;
            if t1 > t2 {
                std::mem::swap(&mut t1, &mut t2);
            }
            t_min = t_min.max(t1);
            t_max = t_max.min(t2);
            if t_min > t_max {
                return None;
            }
        }
        if t_min > 0.0 {
            Some(t_min)
        } else if t_max > 0.0 {
            Some(t_max)
        } else {
            None
        }
    }

    fn normal(&self, punto: [f64; 3]) -> [f64; 3] {
        let epsilon = 1e-4;
        if (punto[0] - self.min[0]).abs() < epsilon {
            [-1.0, 0.0, 0.0]
        } else if (punto[0] - self.max[0]).abs() < epsilon {
            [1.0, 0.0, 0.0]
        } else if (punto[1] - self.min[1]).abs() < epsilon {
            [0.0, -1.0, 0.0]
        } else if (punto[1] - self.max[1]).abs() < epsilon {
            [0.0, 1.0, 0.0]
        } else if (punto[2] - self.min[2]).abs() < epsilon {
            [0.0, 0.0, -1.0]
        } else {
            [0.0, 0.0, 1.0]
        }
    }

    fn uv(&self, punto: [f64; 3]) -> (f64, f64) {
        let normal = self.normal(punto);
        let tam = [
            self.max[0] - self.min[0],
            self.max[1] - self.min[1],
            self.max[2] - self.min[2],
        ];
        if normal[0].abs() > 0.5 {
            ((punto[2] - self.min[2]) / tam[2], (punto[1] - self.min[1]) / tam[1])
        } else if normal[1].abs() > 0.5 {
            ((punto[0] - self.min[0]) / tam[0], (punto[2] - self.min[2]) / tam[2])
        } else {
            ((punto[0] - self.min[0]) / tam[0], (punto[1] - self.min[1]) / tam[1])
        }
    }

    fn textura(&self) -> &dyn Textura {
        self.textura.as_ref()
    }
}
