//! Lista con selección, compartida por todas las vistas.

use ratatui::widgets::TableState;

use super::action::Navigation;

/// Filas que salta una página.
pub const PAGE_SIZE: usize = 20;

/// Qué hacer con la selección al cambiar los elementos.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionPolicy {
    /// Mantener el índice (recortado al nuevo largo): refrescos del mismo contenido.
    Keep,
    /// Volver al primer elemento: contenido nuevo.
    Reset,
}

#[derive(Debug)]
pub struct SelectableList<T> {
    items: Vec<T>,
    table: TableState,
    /// Pedido de dejar la selección centrada en el próximo dibujado.
    centering_requested: bool,
}

impl<T> Default for SelectableList<T> {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            table: TableState::default(),
            centering_requested: false,
        }
    }
}

impl<T> SelectableList<T> {
    pub fn items(&self) -> &[T] {
        &self.items
    }

    pub fn selected(&self) -> Option<&T> {
        self.items.get(self.table.selected()?)
    }

    /// Elementos y estado de la tabla a la vez, para dibujar sin copiar.
    pub fn parts_mut(&mut self) -> (&[T], &mut TableState) {
        (&self.items, &mut self.table)
    }

    pub fn set_items(&mut self, items: Vec<T>, policy: SelectionPolicy) {
        self.items = items;
        let index = match policy {
            SelectionPolicy::Keep => self.table.selected().unwrap_or(0),
            SelectionPolicy::Reset => 0,
        };
        if policy == SelectionPolicy::Reset {
            *self.table.offset_mut() = 0;
        }
        self.select_clamped(index);
    }

    pub fn clear(&mut self) {
        self.set_items(Vec::new(), SelectionPolicy::Reset);
    }

    /// Mueve la selección; devuelve `true` si cambió.
    pub fn navigate(&mut self, navigation: Navigation) -> bool {
        let previous = self.table.selected();
        let current = previous.unwrap_or(0);
        let target = match navigation {
            Navigation::Next => current.saturating_add(1),
            Navigation::Previous => current.saturating_sub(1),
            Navigation::PageDown => current.saturating_add(PAGE_SIZE),
            Navigation::PageUp => current.saturating_sub(PAGE_SIZE),
            Navigation::First => 0,
            Navigation::Last => usize::MAX,
        };
        self.select_clamped(target);
        self.table.selected() != previous
    }

    /// Selecciona el primer elemento que cumpla la condición, si existe.
    pub fn select_where(&mut self, predicate: impl Fn(&T) -> bool) {
        if let Some(index) = self.items.iter().position(predicate) {
            self.table.select(Some(index));
        }
    }

    /// La próxima vez que se dibuje, la selección queda centrada si se puede.
    pub fn request_centering(&mut self) {
        self.centering_requested = true;
    }

    /// Consume el pedido de centrado.
    pub fn take_centering_request(&mut self) -> bool {
        std::mem::take(&mut self.centering_requested)
    }

    fn select_clamped(&mut self, index: usize) {
        let last = self.items.len().checked_sub(1);
        self.table.select(last.map(|last| index.min(last)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn list_of(length: usize) -> SelectableList<usize> {
        let mut list = SelectableList::default();
        list.set_items((0..length).collect(), SelectionPolicy::Reset);
        list
    }

    #[test]
    fn navigation_stays_in_range_and_reports_changes() {
        let mut list = list_of(3);
        assert_eq!(list.selected(), Some(&0));
        assert!(!list.navigate(Navigation::Previous));
        assert!(list.navigate(Navigation::PageDown));
        assert_eq!(list.selected(), Some(&2));
        assert!(list.navigate(Navigation::First));
        assert!(list.navigate(Navigation::Last));
        assert_eq!(list.selected(), Some(&2));
    }

    #[test]
    fn keep_policy_clamps_and_reset_returns_to_top() {
        let mut list = list_of(5);
        list.navigate(Navigation::Last);
        list.set_items(vec![10, 11], SelectionPolicy::Keep);
        assert_eq!(list.selected(), Some(&11));
        list.set_items(vec![20, 21], SelectionPolicy::Reset);
        assert_eq!(list.selected(), Some(&20));
        list.clear();
        assert_eq!(list.selected(), None);
    }

    #[test]
    fn select_where_finds_matching_item() {
        let mut list = list_of(4);
        list.select_where(|item| *item == 3);
        assert_eq!(list.selected(), Some(&3));
        list.select_where(|item| *item == 99);
        assert_eq!(list.selected(), Some(&3));
    }

    #[test]
    fn centering_request_is_consumed_once() {
        let mut list = list_of(2);
        assert!(!list.take_centering_request());
        list.request_centering();
        assert!(list.take_centering_request());
        assert!(!list.take_centering_request());
    }
}
