//! Actualizaciones de la base de datos de MPD. El servidor no informa cuándo empezó ni
//! cuánto falta una actualización: solo si hay una en curso (`updating_db`).

use crate::domain::Status;

/// Qué pasó con la actualización entre dos lecturas sucesivas del estado.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateTransition {
    Started,
    Finished,
}

/// Compara dos estados sucesivos; `None` si la actualización sigue igual.
pub fn update_transition(previous: &Status, current: &Status) -> Option<UpdateTransition> {
    match (
        previous.updating_db.is_some(),
        current.updating_db.is_some(),
    ) {
        (false, true) => Some(UpdateTransition::Started),
        (true, false) => Some(UpdateTransition::Finished),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn status(updating_db: Option<u32>) -> Status {
        Status {
            updating_db,
            ..Status::default()
        }
    }

    #[test]
    fn detects_start_and_finish() {
        let idle = status(None);
        let updating = status(Some(3));
        assert_eq!(
            update_transition(&idle, &updating),
            Some(UpdateTransition::Started)
        );
        assert_eq!(
            update_transition(&updating, &idle),
            Some(UpdateTransition::Finished)
        );
        assert_eq!(update_transition(&updating, &status(Some(4))), None);
        assert_eq!(update_transition(&idle, &idle), None);
    }
}
