use crate::sphere::Sphere;
use crate::textura::BolaBillar;

const RADIO_BOLA: f64 = 0.12;
const ALTURA_MESA: f64 = crate::mesa::ALTURA_PANO;

// Asignacion de color y tipo (completa o rayada)
fn datos_bola(numero: u8) -> ([f64; 3], bool) {
    let color = match numero {
        1 | 9 => [1.0, 0.82, 0.04],
        2 | 10 => [0.03, 0.15, 0.82],
        3 | 11 => [0.88, 0.025, 0.02],
        4 | 12 => [0.42, 0.04, 0.58],
        5 | 13 => [1.0, 0.28, 0.015],
        6 | 14 => [0.02, 0.46, 0.10],
        7 | 15 => [0.43, 0.015, 0.035],
        8 => [0.012, 0.012, 0.015],
        _ => [0.96, 0.95, 0.88],
    };
    (color, numero >= 9)
}

pub fn crear_bolas() -> Vec<Sphere> {
    let altura = ALTURA_MESA + RADIO_BOLA;
    let separacion = RADIO_BOLA * 2.08;
    let mut bolas = Vec::with_capacity(16);

    // Creacion de la bola blanca
    bolas.push(Sphere::new(
        [0.0, altura, -2.65],
        RADIO_BOLA,
        Box::new(BolaBillar::new(0, [0.96, 0.95, 0.88], false)),
    ));

    // Distribucion triangular de las bolas
    let orden = [1, 10, 2, 3, 8, 11, 12, 4, 13, 5, 7, 14, 6, 15, 9];
    let inicio_z = -4.70;
    let mut indice = 0;

    for fila in 0..5 {
        let z = inicio_z - fila as f64 * separacion;
        for columna in 0..=fila {
            let x = (columna as f64 - fila as f64 / 2.0) * separacion;
            let numero = orden[indice];
            let (color, rayada) = datos_bola(numero);
            bolas.push(Sphere::new(
                [x, altura, z],
                RADIO_BOLA,
                Box::new(BolaBillar::new(numero, color, rayada)),
            ));
            indice += 1;
        }
    }

    bolas
}