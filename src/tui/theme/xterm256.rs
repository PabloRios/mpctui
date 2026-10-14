//! El índice más cercano de la paleta de 256 colores de xterm: el cubo 6×6×6
//! (índices 16–231) y la rampa de grises (232–255). Los 16 primeros dependen de la
//! terminal y no se usan.

use super::spec::Rgb;

const CUBE_START: u8 = 16;
/// Valor de cada uno de los seis niveles por canal del cubo.
const CUBE_LEVELS: [u8; 6] = [0, 95, 135, 175, 215, 255];
const CUBE_SIDE: u8 = CUBE_LEVELS.len() as u8;
const GRAY_START: u8 = 232;
const GRAY_COUNT: u8 = 24;
/// Gris `n` de la rampa: `8 + 10 × n`.
const GRAY_FIRST_VALUE: u8 = 8;
const GRAY_STEP: u8 = 10;

pub fn nearest_index(color: Rgb) -> u8 {
    let channels = [color.red, color.green, color.blue];
    let levels = channels.map(nearest_level);
    let cube = Rgb {
        red: CUBE_LEVELS[usize::from(levels[0])],
        green: CUBE_LEVELS[usize::from(levels[1])],
        blue: CUBE_LEVELS[usize::from(levels[2])],
    };
    let cube_index =
        CUBE_START + levels[0] * CUBE_SIDE * CUBE_SIDE + levels[1] * CUBE_SIDE + levels[2];
    let gray_step = nearest_gray(color);
    let gray_value = GRAY_FIRST_VALUE + GRAY_STEP * gray_step;
    let gray = Rgb {
        red: gray_value,
        green: gray_value,
        blue: gray_value,
    };
    if distance(color, gray) < distance(color, cube) {
        GRAY_START + gray_step
    } else {
        cube_index
    }
}

/// El nivel del cubo más cercano a un canal.
fn nearest_level(channel: u8) -> u8 {
    let position = CUBE_LEVELS
        .iter()
        .enumerate()
        .min_by_key(|(_, level)| channel.abs_diff(**level))
        .map_or(0, |(index, _)| index);
    position as u8
}

/// El paso de la rampa de grises más cercano al promedio de los canales.
fn nearest_gray(color: Rgb) -> u8 {
    let average = (u16::from(color.red) + u16::from(color.green) + u16::from(color.blue)) / 3;
    let offset = average.saturating_sub(u16::from(GRAY_FIRST_VALUE));
    let step = (offset + u16::from(GRAY_STEP) / 2) / u16::from(GRAY_STEP);
    step.min(u16::from(GRAY_COUNT - 1)) as u8
}

fn distance(first: Rgb, second: Rgb) -> u32 {
    let squared = |left: u8, right: u8| u32::from(left.abs_diff(right)).pow(2);
    squared(first.red, second.red)
        + squared(first.green, second.green)
        + squared(first.blue, second.blue)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_colors_map_to_their_index() {
        assert_eq!(nearest_index(Rgb::hex(0xff_00_00)), 196);
        assert_eq!(nearest_index(Rgb::hex(0x80_80_80)), 244);
        assert_eq!(nearest_index(Rgb::hex(0x00_00_00)), 16);
        assert_eq!(nearest_index(Rgb::hex(0xff_ff_ff)), 231);
        assert_eq!(nearest_index(Rgb::hex(0x5f_87_af)), 67);
        assert_eq!(nearest_index(Rgb::hex(0x1c_1c_1c)), 234);
    }
}
