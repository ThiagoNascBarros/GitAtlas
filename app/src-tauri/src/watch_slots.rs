// SPDX-License-Identifier: Apache-2.0
//! Um observador de arquivos por repositório aberto (PERF-01).
//!
//! Atualizar um repositório (commit, undo, fetch) relê o histórico, mas não precisa
//! recriar o observador: a pasta raiz é a mesma. Recriar gastava um `git ls-files`
//! e registrava de novo todas as pastas. O observador só é criado ao abrir o
//! repositório e descartado ao fechá-lo.

use std::collections::HashMap;

pub struct WatcherSlots<W> {
    slots: HashMap<u32, W>,
}

impl<W> Default for WatcherSlots<W> {
    fn default() -> Self {
        Self {
            slots: HashMap::new(),
        }
    }
}

impl<W> WatcherSlots<W> {
    /// Garante um observador para `id`: chama `start` só se ainda não existe.
    /// Devolve `true` quando criou um novo. Se `start` falhar, nada fica registrado
    /// e a próxima chamada tenta de novo.
    pub fn ensure(&mut self, id: u32, start: impl FnOnce() -> Option<W>) -> bool {
        if self.slots.contains_key(&id) {
            return false;
        }
        match start() {
            Some(w) => {
                self.slots.insert(id, w);
                true
            }
            None => false,
        }
    }

    /// `true` quando `id` já tem observador registrado.
    pub fn contains(&self, id: u32) -> bool {
        self.slots.contains_key(&id)
    }

    /// Descarta o observador do repositório fechado (o que para a observação).
    pub fn remove(&mut self, id: u32) -> Option<W> {
        self.slots.remove(&id)
    }

    pub fn len(&self) -> usize {
        self.slots.len()
    }

    pub fn is_empty(&self) -> bool {
        self.slots.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    /// Stand-in for starting a watcher: counts how many times it ran.
    fn start(starts: &Cell<u32>) -> Option<u32> {
        starts.set(starts.get() + 1);
        Some(starts.get())
    }

    #[test]
    fn starts_once_per_repository_across_refreshes() {
        let mut slots = WatcherSlots::default();
        let starts = Cell::new(0);
        assert!(slots.ensure(1, || start(&starts)));
        // Atualizações seguintes do mesmo repositório não recriam o observador.
        assert!(!slots.ensure(1, || start(&starts)));
        assert!(!slots.ensure(1, || start(&starts)));
        assert_eq!(starts.get(), 1);
        assert_eq!(slots.len(), 1);
    }

    #[test]
    fn each_repository_has_its_own_watcher() {
        let mut slots = WatcherSlots::default();
        assert!(slots.ensure(1, || Some("a")));
        assert!(slots.ensure(2, || Some("b")));
        assert_eq!(slots.len(), 2);
    }

    #[test]
    fn failed_start_is_not_remembered() {
        let mut slots: WatcherSlots<u8> = WatcherSlots::default();
        assert!(!slots.ensure(1, || None));
        assert!(slots.is_empty());
        assert!(slots.ensure(1, || Some(9)));
    }

    #[test]
    fn closing_frees_the_slot_and_reopening_starts_again() {
        let mut slots = WatcherSlots::default();
        assert!(slots.ensure(1, || Some(1)));
        assert_eq!(slots.remove(1), Some(1));
        assert_eq!(slots.remove(1), None);
        assert!(slots.ensure(1, || Some(2)));
    }
}
