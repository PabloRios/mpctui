//! Cascada (espectrograma): historial de columnas de bandas que avanza a ritmo fijo.

use std::collections::VecDeque;
use std::time::Duration;

use crate::domain::WATERFALL_COLUMNS_PER_SECOND;

/// Tiempo entre una columna y la siguiente.
pub const WATERFALL_COLUMN_INTERVAL: Duration =
    Duration::from_millis(1_000 / WATERFALL_COLUMNS_PER_SECOND as u64);

#[derive(Debug, Default)]
pub struct Waterfall {
    /// De la más vieja a la más nueva.
    columns: VecDeque<Vec<f32>>,
    /// Tiempo acumulado desde la última columna.
    pending: Duration,
}

impl Waterfall {
    /// Cuántas columnas tocan después de `elapsed`; el resto queda para la próxima.
    pub fn columns_due(&mut self, elapsed: Duration) -> usize {
        self.pending += elapsed;
        let mut due = 0;
        while self.pending >= WATERFALL_COLUMN_INTERVAL {
            self.pending -= WATERFALL_COLUMN_INTERVAL;
            due += 1;
        }
        due
    }

    /// Agrega una columna y conserva las `capacity` más nuevas. Si cambió la cantidad
    /// de bandas, lo anterior ya no se puede dibujar y se descarta.
    pub fn push(&mut self, levels: Vec<f32>, capacity: usize) {
        if self
            .columns
            .back()
            .is_some_and(|last| last.len() != levels.len())
        {
            self.columns.clear();
        }
        self.columns.push_back(levels);
        self.keep_newest(capacity);
    }

    /// Si la terminal se achica, se quedan las columnas más nuevas.
    pub fn keep_newest(&mut self, capacity: usize) {
        while self.columns.len() > capacity {
            self.columns.pop_front();
        }
    }

    pub fn columns(&self) -> Vec<Vec<f32>> {
        self.columns.iter().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FRAME: Duration = Duration::from_millis(50);

    #[test]
    fn a_column_is_due_every_hundred_milliseconds() {
        let mut waterfall = Waterfall::default();
        assert_eq!(waterfall.columns_due(FRAME), 0);
        assert_eq!(waterfall.columns_due(FRAME), 1);
        assert_eq!(waterfall.columns_due(Duration::from_millis(250)), 2);
        assert_eq!(waterfall.columns_due(FRAME), 1);
    }

    #[test]
    fn shrinking_keeps_the_newest_columns() {
        let mut waterfall = Waterfall::default();
        for value in [0.1, 0.2, 0.3, 0.4] {
            waterfall.push(vec![value], 10);
        }
        waterfall.keep_newest(2);
        assert_eq!(waterfall.columns(), [vec![0.3], vec![0.4]]);
        waterfall.push(vec![0.5], 2);
        assert_eq!(waterfall.columns(), [vec![0.4], vec![0.5]]);
    }

    #[test]
    fn a_new_band_count_starts_over() {
        let mut waterfall = Waterfall::default();
        waterfall.push(vec![0.1, 0.2], 10);
        waterfall.push(vec![0.1, 0.2, 0.3], 10);
        assert_eq!(waterfall.columns(), [vec![0.1, 0.2, 0.3]]);
    }
}
