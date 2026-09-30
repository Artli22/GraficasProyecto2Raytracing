use raylib::prelude::*;
use std::f64::consts::PI;

/// Color base (albedo) en espacio [0.0, 1.0] por canal RGB.
/// Queda separado del framebuffer para que este mismo archivo pueda
/// incorporar mas adelante reflexion dinamica, refraccion, mapas de
/// normales, etc. sin tocar la logica de dibujado.
pub type Albedo = [f64; 3];

/// Requiere Send + Sync por la misma razon que `Objeto`: el render en
/// paralelo (rayon) comparte referencias a las texturas entre hilos.
pub trait Textura: Send + Sync {
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

/// Ruido "hash" pseudo-aleatorio en una retícula entera, base para
/// construir el value noise. Determinista y sin dependencias externas.
/// Solo se invoca al construir la tabla precalculada (una vez), nunca
/// por pixel, ya que usa sin(), una funcion trascendental costosa.
fn hash_ruido(x: f64, y: f64) -> f64 {
    let n = (x * 127.1 + y * 311.7).sin() * 43758.5453;
    n.fract().abs()
}

// Lado de la tabla de ruido precalculada. El dominio es toroidal (se repite
// cada TAMANO_TABLA_RUIDO unidades de reticula), asi que el tejido nunca
// muestra una costura visible al hacer wrap.
const TAMANO_TABLA_RUIDO: i64 = 256;

/// Genera, una sola vez, la tabla de valores hash sobre una reticula
/// TAMANO_TABLA_RUIDO x TAMANO_TABLA_RUIDO. Este es el unico lugar del
/// programa donde se llama sin() para el ruido de la tela.
fn generar_tabla_ruido() -> Vec<f64> {
    let n = TAMANO_TABLA_RUIDO as usize;
    let mut tabla = Vec::with_capacity(n * n);
    for y in 0..n {
        for x in 0..n {
            tabla.push(hash_ruido(x as f64, y as f64));
        }
    }
    tabla
}

fn tabla_lookup(tabla: &[f64], x: i64, y: i64) -> f64 {
    let n = TAMANO_TABLA_RUIDO;
    let xi = x.rem_euclid(n) as usize;
    let yi = y.rem_euclid(n) as usize;
    tabla[yi * n as usize + xi]
}

/// Value noise con interpolacion suave (smoothstep) entre las 4 esquinas
/// de la celda de la reticula que contiene (x, y). Lee los valores hash
/// de la tabla precalculada (simples lookups + aritmetica, sin sin())
/// en vez de recalcularlos por pixel.
fn valor_ruido(tabla: &[f64], x: f64, y: f64) -> f64 {
    let x0 = x.floor();
    let y0 = y.floor();
    let fx = x - x0;
    let fy = y - y0;
    let ix0i = x0 as i64;
    let iy0i = y0 as i64;

    let h00 = tabla_lookup(tabla, ix0i, iy0i);
    let h10 = tabla_lookup(tabla, ix0i + 1, iy0i);
    let h01 = tabla_lookup(tabla, ix0i, iy0i + 1);
    let h11 = tabla_lookup(tabla, ix0i + 1, iy0i + 1);

    // Interpolacion suave (evita las "costuras" lineales de una interpolacion lineal simple).
    let sx = fx * fx * (3.0 - 2.0 * fx);
    let sy = fy * fy * (3.0 - 2.0 * fy);

    let ix0 = h00 + (h10 - h00) * sx;
    let ix1 = h01 + (h11 - h01) * sx;
    ix0 + (ix1 - ix0) * sy
}

/// Ruido fractal (fbm): suma varias octavas de `valor_ruido` a frecuencias
/// crecientes y amplitudes decrecientes. Esto agrega detalle fino sobre
/// una variacion mas amplia, tal como se ve el grano irregular de una tela.
/// Cada octava es ahora solo lookups + interpolacion, sin trigonometria.
fn fbm(tabla: &[f64], x: f64, y: f64, octavas: u32) -> f64 {
    let mut total = 0.0;
    let mut amplitud = 0.5;
    let mut frecuencia = 1.0;
    let mut amplitud_maxima = 0.0;

    for _ in 0..octavas {
        total += valor_ruido(tabla, x * frecuencia, y * frecuencia) * amplitud;
        amplitud_maxima += amplitud;
        amplitud *= 0.5;
        frecuencia *= 2.0;
    }

    total / amplitud_maxima
}

/// Simula el tejido de un paño de billar: grano fino e irregular generado
/// con ruido fractal, sin ningun patron geometrico repetitivo. Sin
/// highlight especular, ya que la tela es mate. El ruido se precalcula
/// una sola vez en `new()`; renderizar solo hace lookups en la tabla,
/// eliminando por completo el costo de sin() del bucle por pixel.
pub struct Fieltro {
    pub color_base: Albedo,
    pub escala: f64,
    pub variacion: f64,
    tabla_ruido: Vec<f64>,
}

impl Fieltro {
    pub fn new(color_base: Albedo, escala: f64, variacion: f64) -> Self {
        Fieltro {
            color_base,
            escala,
            variacion,
            tabla_ruido: generar_tabla_ruido(),
        }
    }
}

impl Textura for Fieltro {
    fn albedo(&self, u: f64, v: f64) -> Albedo {
        // fbm regresa un valor centrado alrededor de ~0.5; lo usamos para
        // aclarar/oscurecer el color base de forma organica e irregular.
        let ruido = fbm(&self.tabla_ruido, u * self.escala, v * self.escala, 4);
        let factor = 1.0 + (ruido - 0.5) * self.variacion;

        [
            (self.color_base[0] * factor).clamp(0.0, 1.0),
            (self.color_base[1] * factor).clamp(0.0, 1.0),
            (self.color_base[2] * factor).clamp(0.0, 1.0),
        ]
    }

    fn brillo(&self) -> f64 {
        0.0
    }
}

/// Madera tallada/tratada: veta direccional procedural sobre un color base,
/// mas la especularidad del barniz. Reutiliza la misma tabla de ruido
/// precalculada y las mismas funciones (`fbm`, `valor_ruido`, `tabla_lookup`)
/// que `Fieltro`, asi que tampoco llama sin() en el bucle por pixel.
///
/// La diferencia frente al ruido isotropico de la tela es la anisotropia:
/// se usa una frecuencia baja a lo largo de la veta (`escala_veta`) y una
/// frecuencia alta en la direccion perpendicular (`escala_ancho`), lo que
/// estira el mismo ruido en lineas alargadas en vez de manchas uniformes.
pub struct TexturaMadera {
    pub color_base: Albedo,
    pub color_veta: Albedo,
    pub escala_veta: f64,
    pub escala_ancho: f64,
    pub brillo: f64,
    tabla_ruido: Vec<f64>,
}

impl TexturaMadera {
    pub fn new(
        color_base: Albedo,
        color_veta: Albedo,
        escala_veta: f64,
        escala_ancho: f64,
        brillo: f64,
    ) -> Self {
        TexturaMadera {
            color_base,
            color_veta,
            escala_veta,
            escala_ancho,
            brillo,
            tabla_ruido: generar_tabla_ruido(),
        }
    }
}

impl Textura for TexturaMadera {
    fn albedo(&self, u: f64, v: f64) -> Albedo {
        // Frecuencia distinta por eje: estira el mismo ruido en vetas
        // alargadas en vez de manchas isotropicas como en la tela.
        let ruido = fbm(
            &self.tabla_ruido,
            u * self.escala_ancho,
            v * self.escala_veta,
            4,
        );

        // Eleva el contraste para que las vetas oscuras se vean como
        // lineas mas definidas en vez de un degradado suave. Se usa un
        // exponente ENTERO (powi) en vez de fraccionario (powf): powf con
        // un exponente no entero requiere exp(ln(x)*n) por dentro -otra
        // funcion trascendental tan cara como el sin() que ya eliminamos-
        // mientras que powi se resuelve con simples multiplicaciones.
        let intensidad_veta = ruido.powi(3).clamp(0.0, 1.0);

        [
            self.color_base[0] * (1.0 - intensidad_veta) + self.color_veta[0] * intensidad_veta,
            self.color_base[1] * (1.0 - intensidad_veta) + self.color_veta[1] * intensidad_veta,
            self.color_base[2] * (1.0 - intensidad_veta) + self.color_veta[2] * intensidad_veta,
        ]
    }

    fn brillo(&self) -> f64 {
        self.brillo
    }
}

/// Textura procedural de una bola de billar. El numero 0 identifica la blanca.
pub struct BolaBillar {
    pub numero: u8,
    pub color: Albedo,
    pub rayada: bool,
}

impl BolaBillar {
    pub fn new(numero: u8, color: Albedo, rayada: bool) -> Self {
        BolaBillar { numero, color, rayada }
    }
}

impl Textura for BolaBillar {
    fn albedo(&self, u: f64, v: f64) -> Albedo {
        if self.numero == 0 {
            return [0.96, 0.95, 0.88];
        }

        let blanco = [0.96, 0.95, 0.88];
        let mut resultado = if self.rayada && !(0.34..=0.66).contains(&v) {
            blanco
        } else {
            self.color
        };

        // Dos discos numerados en lados opuestos de la esfera.
        for centro_u in [0.0, 0.5] {
            let du = (u - centro_u + 0.5).rem_euclid(1.0) - 0.5;
            let dv = v - 0.5;

            let radio_horizontal = 0.055;
            let radio_vertical = 0.105;

            let x = du / radio_horizontal;
            let y = dv / radio_vertical;

            if x * x + y * y <= 1.0 {
                resultado = blanco;

                if numero_visible(self.numero, x, y) {
                    return [0.01, 0.01, 0.01];
                }
            }
        }
        resultado
    }

    fn brillo(&self) -> f64 {
        100.0
    }
}

fn pixel_digito(digito: u8, x: f64, y: f64) -> bool {
    const DIGITOS: [[u8; 5]; 10] = [
        [0b111, 0b101, 0b101, 0b101, 0b111],
        [0b010, 0b010, 0b010, 0b110, 0b010],
        [0b111, 0b100, 0b111, 0b001, 0b111],
        [0b111, 0b001, 0b011, 0b001, 0b111],
        [0b001, 0b001, 0b111, 0b101, 0b101],
        [0b111, 0b001, 0b111, 0b100, 0b111],
        [0b111, 0b101, 0b111, 0b100, 0b111],
        [0b001, 0b001, 0b011, 0b001, 0b111],
        [0b111, 0b101, 0b111, 0b101, 0b111],
        [0b001, 0b001, 0b111, 0b101, 0b111],
    ];

    if digito > 9 || !(-0.5..=0.5).contains(&x) || !(-0.75..=0.75).contains(&y) {
        return false;
    }

    let columna = ((x + 0.5) * 3.0).floor().clamp(0.0, 2.0) as usize;
    let fila = ((0.75 - y) / 1.5 * 5.0).floor().clamp(0.0, 4.0) as usize;
    DIGITOS[digito as usize][fila] & (1 << (2 - columna)) != 0
}

fn numero_visible(numero: u8, x: f64, y: f64) -> bool {
    if numero < 10 {
        pixel_digito(numero, x * 0.48, y * 0.95)
    } else {
        pixel_digito(numero / 10, (x + 0.32) / 0.55, y * 0.60)
            || pixel_digito(numero % 10, (x - 0.32) / 0.55, y * 0.82)
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

/// Iluminacion Blinn-Phong producida por una luz puntual fija en el mundo.
pub fn sombrear_puntual(
    ray_dir: [f64; 3],
    punto: [f64; 3],
    normal: [f64; 3],
    posicion_luz: [f64; 3],
    color_luz: Albedo,
    albedo: Albedo,
    shininess: f64,
) -> Color {
    let hacia_luz = [
        posicion_luz[0] - punto[0],
        posicion_luz[1] - punto[1],
        posicion_luz[2] - punto[2],
    ];
    let distancia_cuadrada = hacia_luz[0] * hacia_luz[0]
        + hacia_luz[1] * hacia_luz[1]
        + hacia_luz[2] * hacia_luz[2];
    let distancia = distancia_cuadrada.sqrt().max(1e-9);
    let light_dir = [
        hacia_luz[0] / distancia,
        hacia_luz[1] / distancia,
        hacia_luz[2] / distancia,
    ];

    let diffuse = (normal[0] * light_dir[0]
        + normal[1] * light_dir[1]
        + normal[2] * light_dir[2])
        .max(0.0);
    let view_dir = [-ray_dir[0], -ray_dir[1], -ray_dir[2]];
    let half = [
        light_dir[0] + view_dir[0],
        light_dir[1] + view_dir[1],
        light_dir[2] + view_dir[2],
    ];
    let half_len = (half[0] * half[0] + half[1] * half[1] + half[2] * half[2])
        .sqrt()
        .max(1e-9);
    let half_norm = [half[0] / half_len, half[1] / half_len, half[2] / half_len];
    let spec_dot = (normal[0] * half_norm[0]
        + normal[1] * half_norm[1]
        + normal[2] * half_norm[2])
        .max(0.0);
    let specular = if shininess > 0.0 {
        spec_dot.powf(shininess)
    } else {
        0.0
    };

    let atenuacion = 1.0 / (1.0 + 0.025 * distancia_cuadrada);
    let ambiente = 0.075;
    let canal = |i: usize| {
        ((albedo[i] * ambiente
            + albedo[i] * color_luz[i] * diffuse * atenuacion
            + color_luz[i] * specular * 0.65 * atenuacion)
            .clamp(0.0, 1.0)
            * 255.0) as u8
    };

    Color::new(canal(0), canal(1), canal(2), 255)
}