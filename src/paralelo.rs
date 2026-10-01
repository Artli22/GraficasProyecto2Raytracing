use raylib::prelude::Color;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self, Scope};

/// Se copia por bloque; no contiene handles de raylib ni referencias mutables.
#[derive(Clone, Copy)]
pub struct Camara {
    pub posicion: [f64; 3],
    pub forward: [f64; 3],
    pub right: [f64; 3],
    pub up: [f64; 3],
}

struct Trabajo {
    camara: Camara,
    fila: usize,
    filas: usize,
    pixeles: Vec<Color>,
}

enum Resultado {
    Listo { trabajador: usize, trabajo: Trabajo },
    Fallo { trabajador: usize },
}

/// Hilos persistentes durante todo el bucle de ventana. Solo usa std y Color.
/// Cada trabajador posee su buffer; se mueve por canales y se reutiliza.
/// El principal entrega el siguiente bloque al primero que termina.
pub struct RenderParalelo {
    ancho: usize,
    alto: usize,
    filas_bloque: usize,
    envios: Vec<Sender<Trabajo>>,
    resultados: Receiver<Resultado>,
    buffers: Vec<Option<Vec<Color>>>,
}

impl RenderParalelo {
    pub fn new<'scope, 'env, F>(
        scope: &'scope Scope<'scope, 'env>,
        ancho: usize,
        alto: usize,
        trabajadores: usize,
        filas_bloque: usize,
        pixel: &'scope F,
    ) -> Self
    where
        F: Fn(usize, usize, &Camara) -> Color + Sync + 'scope,
    {
        assert!(ancho > 0 && alto > 0 && trabajadores > 0 && filas_bloque > 0);
        let filas_bloque = filas_bloque.min(alto);
        let cantidad = trabajadores.min(alto.div_ceil(filas_bloque));
        let (salida, resultados) = mpsc::channel();
        let mut envios = Vec::with_capacity(cantidad);
        let mut buffers = Vec::with_capacity(cantidad);
        for trabajador in 0..cantidad {
            let (entrada, receptor) = mpsc::channel::<Trabajo>();
            let salida = salida.clone();
            scope.spawn(move || {
                while let Ok(mut trabajo) = receptor.recv() {
                    // Un fallo se comunica: el principal no queda esperando para siempre.
                    let resultado = catch_unwind(AssertUnwindSafe(|| {
                        for fila in 0..trabajo.filas {
                            for x in 0..ancho {
                                trabajo.pixeles[fila * ancho + x] =
                                    pixel(x, trabajo.fila + fila, &trabajo.camara);
                            }
                        }
                    }));
                    if resultado.is_err() {
                        let _ = salida.send(Resultado::Fallo { trabajador });
                        break;
                    }
                    if salida
                        .send(Resultado::Listo {
                            trabajador,
                            trabajo,
                        })
                        .is_err()
                    {
                        break;
                    }
                }
            });
            envios.push(entrada);
            buffers.push(Some(vec![Color::new(0, 0, 0, 255); ancho * filas_bloque]));
        }
        drop(salida);
        Self {
            ancho,
            alto,
            filas_bloque,
            envios,
            resultados,
            buffers,
        }
    }

    pub fn renderizar(&mut self, camara: Camara, destino: &mut [Color]) -> Result<(), String> {
        if destino.len() != self.ancho * self.alto {
            return Err("El framebuffer no coincide con las dimensiones del render".into());
        }
        let mut siguiente = 0;
        let mut pendientes = 0;
        for id in 0..self.envios.len() {
            if siguiente >= self.alto {
                break;
            }
            let pixeles = self.buffers[id]
                .take()
                .ok_or("El render anterior fallo; recrea los trabajadores")?;
            let filas = self.filas_bloque.min(self.alto - siguiente);
            self.envios[id]
                .send(Trabajo {
                    camara,
                    fila: siguiente,
                    filas,
                    pixeles,
                })
                .map_err(|_| "Un trabajador se desconecto")?;
            siguiente += filas;
            pendientes += 1;
        }
        while pendientes > 0 {
            match self
                .resultados
                .recv()
                .map_err(|_| "No quedan trabajadores activos")?
            {
                Resultado::Fallo { trabajador } => {
                    return Err(format!("El trabajador {trabajador} fallo al renderizar"));
                }
                Resultado::Listo {
                    trabajador,
                    mut trabajo,
                } => {
                    let inicio = trabajo.fila * self.ancho;
                    let cantidad = trabajo.filas * self.ancho;
                    destino[inicio..inicio + cantidad]
                        .copy_from_slice(&trabajo.pixeles[..cantidad]);
                    if siguiente < self.alto {
                        trabajo.fila = siguiente;
                        trabajo.filas = self.filas_bloque.min(self.alto - siguiente);
                        siguiente += trabajo.filas;
                        self.envios[trabajador]
                            .send(trabajo)
                            .map_err(|_| "Un trabajador se desconecto")?;
                    } else {
                        self.buffers[trabajador] = Some(trabajo.pixeles);
                        pendientes -= 1;
                    }
                }
            }
        }
        Ok(())
    }
}

impl Drop for RenderParalelo {
    fn drop(&mut self) {
        // Desconecta los receptores y despierta los hilos ociosos antes del join del Scope.
        self.envios.clear();
    }
}

pub fn trabajadores_disponibles() -> usize {
    thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn camara(valor: f64) -> Camara {
        Camara {
            posicion: [valor, 0.0, 0.0],
            forward: [0.0; 3],
            right: [0.0; 3],
            up: [0.0; 3],
        }
    }
    #[test]
    fn frames_consecutivos_y_bloque_incompleto() {
        let pixel =
            |x: usize, y: usize, c: &Camara| Color::new(x as u8, y as u8, c.posicion[0] as u8, 255);
        for trabajadores in [1, 2, 4, 32] {
            thread::scope(|scope| {
                let mut pool = RenderParalelo::new(scope, 17, 19, trabajadores, 8, &pixel);
                let mut destino = vec![Color::new(0, 0, 0, 0); 17 * 19];
                for frame in 0..4 {
                    pool.renderizar(camara(frame as f64), &mut destino).unwrap();
                    for (i, c) in destino.iter().enumerate() {
                        assert_eq!(
                            [c.r, c.g, c.b, c.a],
                            [(i % 17) as u8, (i / 17) as u8, frame, 255]
                        );
                    }
                }
            });
        }
    }
    #[test]
    fn cierre_sin_frames_y_error_de_dimension() {
        let pixel = |_: usize, _: usize, _: &Camara| Color::new(0, 0, 0, 255);
        thread::scope(|scope| {
            let mut pool = RenderParalelo::new(scope, 8, 8, 2, 4, &pixel);
            assert!(pool.renderizar(camara(0.0), &mut []).is_err());
        });
    }
    #[test]
    fn panic_de_trabajador_se_reporta_sin_bloqueo() {
        let pixel = |_: usize, _: usize, _: &Camara| -> Color { panic!("fallo de prueba") };
        thread::scope(|scope| {
            let mut pool = RenderParalelo::new(scope, 8, 8, 2, 4, &pixel);
            let mut destino = vec![Color::new(0, 0, 0, 255); 64];
            assert!(pool.renderizar(camara(0.0), &mut destino).is_err());
        });
    }
}
