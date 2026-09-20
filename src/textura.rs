use raylib::prelude::*;
use std::f64::consts::PI;

/// Color base (albedo) en espacio [0.0, 1.0] por canal RGB.
/// Queda separado del framebuffer para que este mismo archivo pueda
/// incorporar mas adelante reflexion dinamica, refraccion, mapas de
/// normales, etc. sin tocar la logica de dibujado.
pub type Albedo = [f64; 3];

pub trait Textura {
    /// Devuelve el albedo en las coordenadas UV (u, v), ambas en [0.0, 1.0].
    fn albedo(&self, u: f64, v: f64) -> Albedo;
    /// Ancho del highlight especular; 0.0 significa sin brillo.
    fn brillo(&self) -> f64 {
        0.0
    }
}

/// Textura de color plano, sin variacion espacial.
pub struct ColorSolido {
    pub color: Albedo,
}

impl ColorSolido {
    pub fn new(color: Albedo) -> Self {
        ColorSolido { color }
    }
}

impl Textura for ColorSolido {
    fn albedo(&self, _u: f64, _v: f64) -> Albedo {
        self.color
    }
}

/// Canica: color vidrioso/brillante con especularidad pronunciada.
pub struct Canica {
    pub color: Albedo,
    pub brillo: f64,
}

impl Canica {
    pub fn new(color: Albedo, brillo: f64) -> Self {
        Canica { color, brillo }
    }
}

impl Textura for Canica {
    fn albedo(&self, _u: f64, _v: f64) -> Albedo {
        self.color
    }
    fn brillo(&self) -> f64 {
        self.brillo
    }
}

/// Patron de tablero de ajedrez procedural, util para ver el mapeo UV.
pub struct Tablero {
    pub color_a: Albedo,
    pub color_b: Albedo,
    pub escala: f64,
}

impl Tablero {
    pub fn new(color_a: Albedo, color_b: Albedo, escala: f64) -> Self {
        Tablero { color_a, color_b, escala }
    }
}

impl Textura for Tablero {
    fn albedo(&self, u: f64, v: f64) -> Albedo {
        let fila = (u * self.escala).floor() as i64;
        let columna = (v * self.escala).floor() as i64;
        if (fila + columna) % 2 == 0 { self.color_a } else { self.color_b }
    }
}

/// Textura cargada desde un archivo de imagen (PNG, JPG, etc.).
pub struct ImagenTextura {
    ancho: i32,
    alto: i32,
    pixeles: Vec<Albedo>,
}

impl ImagenTextura {
    pub fn load(path: &str) -> Result<Self, String> {
        let mut imagen = Image::load_image(path).map_err(|e| e.to_string())?;
        let ancho = imagen.width();
        let alto = imagen.height();
        let mut pixeles = Vec::with_capacity((ancho * alto) as usize);
        for y in 0..alto {
            for x in 0..ancho {
                let color = imagen.get_color(x, y);
                pixeles.push([
                    color.r as f64 / 255.0,
                    color.g as f64 / 255.0,
                    color.b as f64 / 255.0,
                ]);
            }
        }
        Ok(ImagenTextura { ancho, alto, pixeles })
    }
}

impl Textura for ImagenTextura {
    fn albedo(&self, u: f64, v: f64) -> Albedo {
        let u = u.rem_euclid(1.0);
        let v = v.rem_euclid(1.0);
        let x = ((u * self.ancho as f64) as i32).clamp(0, self.ancho - 1);
        let y = (((1.0 - v) * self.alto as f64) as i32).clamp(0, self.alto - 1);
        self.pixeles[(y * self.ancho + x) as usize]
    }
}

/// Mapeo esferico: convierte un punto de la superficie (relativo al centro)
/// en coordenadas UV, tal como se hace en el raytracing clasico.
pub fn uv_esfera(punto_local: [f64; 3], radio: f64) -> (f64, f64) {
    let x = punto_local[0] / radio;
    let y = punto_local[1] / radio;
    let z = punto_local[2] / radio;
    let u = 0.5 + x.atan2(z) / (2.0 * std::f64::consts::PI);
    let v = 0.5 - y.asin() / std::f64::consts::PI;
    (u, v)
}

/// Aplica iluminacion Phong: difusa + especularidad para efecto vidrioso.
/// ray_dir: direccion normalizacion del rayo (hacia adelante desde camara)
/// normal: normal del punto de impacto
/// light_dir: direccion normalizacion de la luz
/// albedo: color base de la textura
/// shininess: controla el ancho del highlight (valores altos = pico agudo)
pub fn sombrear(
    ray_dir: [f64; 3],
    normal: [f64; 3],
    light_dir: [f64; 3],
    albedo: Albedo,
    shininess: f64,
) -> Color {
    // Difusión clásica
    let diffuse = (normal[0] * light_dir[0] + normal[1] * light_dir[1] + normal[2] * light_dir[2]).max(0.0);

    // Especularidad Blinn-Phong: half-vector entre luz y vista
    let neg_ray = [-ray_dir[0], -ray_dir[1], -ray_dir[2]]; // hacia la cámara
    let half = [
        light_dir[0] + neg_ray[0],
        light_dir[1] + neg_ray[1],
        light_dir[2] + neg_ray[2],
    ];
    let half_len = (half[0] * half[0] + half[1] * half[1] + half[2] * half[2]).sqrt();
    let half_norm = [half[0] / half_len, half[1] / half_len, half[2] / half_len];

    let spec_dot = (normal[0] * half_norm[0] + normal[1] * half_norm[1] + normal[2] * half_norm[2]).max(0.0);
    let specular = if shininess > 0.0 { spec_dot.powf(shininess) } else { 0.0 };

    // Combinar: componente difusa teñida + highlight blanco
    let r = (albedo[0] * diffuse + specular * 0.5).clamp(0.0, 1.0) * 255.0;
    let g = (albedo[1] * diffuse + specular * 0.5).clamp(0.0, 1.0) * 255.0;
    let b = (albedo[2] * diffuse + specular * 0.5).clamp(0.0, 1.0) * 255.0;

    Color::new(r as u8, g as u8, b as u8, 255)
}

