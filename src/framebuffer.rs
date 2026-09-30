use raylib::prelude::*;

pub struct Framebuffer {
    pub width: i32,
    pub height: i32,
    buffer: Vec<Color>,
    profundidad: Vec<f64>,
}

impl Framebuffer {
    pub fn new(width: i32, height: i32, background: Color) -> Self {
        Framebuffer {
            width,
            height,
            buffer: vec![background; (width * height) as usize],
            // El buffer de profundidad arranca en infinito: nada es "mas cercano" todavia.
            profundidad: vec![f64::INFINITY; (width * height) as usize],
        }
    }

    /// Acceso exclusivo a los colores para repartir filas entre trabajadores.
    pub fn pixels_mut(&mut self) -> &mut [Color] {
        &mut self.buffer
    }

    /// Reinicia color y profundidad antes de renderizar un nuevo frame.
    pub fn clear(&mut self, background: Color) {
        self.buffer.fill(background);
        self.profundidad.fill(f64::INFINITY);
    }

    pub fn set_pixel(&mut self, x: i32, y: i32, color: Color) {
        if x >= 0 && x < self.width && y >= 0 && y < self.height {
            self.buffer[(y * self.width + x) as usize] = color;
        }
    }

    /// Solo pinta el pixel si `depth` esta mas cerca de la camara que lo ya almacenado;
    /// de lo contrario el fragmento se descarta.
    pub fn set_pixel_depth(&mut self, x: i32, y: i32, color: Color, depth: f64) {
        if x >= 0 && x < self.width && y >= 0 && y < self.height {
            let idx = (y * self.width + x) as usize;
            if depth < self.profundidad[idx] {
                self.buffer[idx] = color;
                self.profundidad[idx] = depth;
            }
        }
    }

    pub fn draw<D: RaylibDraw>(&self, d: &mut D) {
        for y in 0..self.height {
            for x in 0..self.width {
                let color = self.buffer[(y * self.width + x) as usize];
                d.draw_pixel(x, y, color);
            }
        }
    }

    /// Vista de los pixeles como bytes RGBA, lista para subir a una textura de GPU
    /// (evita el costo de invocar draw_pixel por cada pixel via FFI).
    pub fn as_bytes(&self) -> &[u8] {
        let len = self.buffer.len() * std::mem::size_of::<Color>();
        unsafe { std::slice::from_raw_parts(self.buffer.as_ptr() as *const u8, len) }
    }
}
