//! Toy manager for switching between toys

use std::collections::HashMap;

use super::Toy;

pub struct ToyManager {
    toys: HashMap<String, Box<dyn Toy>>,
    active_toy_id: String,
}

impl ToyManager {
    #[must_use]
    pub fn new() -> Self {
        let toys = super::registry::create_all_toys();
        let active_toy_id = super::registry::toy_id_order()[0].to_string();

        Self {
            toys,
            active_toy_id,
        }
    }

    /// Returns the currently active toy.
    ///
    /// # Panics
    ///
    /// Panics if the active toy ID does not correspond to a registered toy.
    #[must_use]
    pub fn active_toy(&self) -> &dyn Toy {
        self.toys
            .get(&self.active_toy_id)
            .map(std::convert::AsRef::as_ref)
            .expect("Active toy should always exist")
    }

    /// Returns the currently active toy as a mutable reference.
    ///
    /// # Panics
    ///
    /// Panics if the active toy ID does not correspond to a registered toy.
    pub fn active_toy_mut(&mut self) -> &mut dyn Toy {
        self.toys
            .get_mut(&self.active_toy_id)
            .map(std::convert::AsMut::as_mut)
            .expect("Active toy should always exist")
    }

    pub fn switch_to(&mut self, id: &str) {
        if self.toys.contains_key(id) {
            self.active_toy_id = id.to_string();
        }
    }

    pub fn reset_active(&mut self) {
        if let Some(toy) = self.toys.get_mut(&self.active_toy_id) {
            toy.reset();
        }
    }

    #[must_use]
    pub fn toy_list(&self) -> Vec<(&str, &str)> {
        super::registry::toy_id_order()
            .into_iter()
            .filter_map(|id| {
                let toy = self.toys.get(id)?;
                Some((toy.id(), toy.name()))
            })
            .collect()
    }

    #[must_use]
    pub fn active_toy_id(&self) -> &str {
        &self.active_toy_id
    }

    #[must_use]
    pub fn active_toy_name(&self) -> &str {
        self.active_toy().name()
    }
}

impl Default for ToyManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_defaults_to_polytopes() {
        let mgr = ToyManager::new();
        assert_eq!(mgr.active_toy_id(), "polytopes");
    }

    #[test]
    fn test_switch_to_known_toy() {
        let mut mgr = ToyManager::new();
        let ids: Vec<String> = mgr
            .toy_list()
            .into_iter()
            .map(|(id, _)| id.to_string())
            .collect();
        if ids.len() > 1 {
            mgr.switch_to(&ids[0]);
            assert_eq!(mgr.active_toy_id(), ids[0]);
        }
    }

    #[test]
    fn test_switch_to_unknown_ignored() {
        let mut mgr = ToyManager::new();
        mgr.switch_to("nonexistent_toy");
        assert_eq!(mgr.active_toy_id(), "polytopes");
    }

    #[test]
    fn test_active_toy_has_name() {
        let mgr = ToyManager::new();
        assert!(!mgr.active_toy_name().is_empty());
    }

    #[test]
    fn test_toy_list_nonempty() {
        let mgr = ToyManager::new();
        let list = mgr.toy_list();
        assert!(!list.is_empty());
        assert!(list.iter().any(|(id, _)| *id == "polytopes"));
    }

    #[test]
    fn test_reset_active_does_not_panic() {
        let mut mgr = ToyManager::new();
        mgr.reset_active();
        assert_eq!(mgr.active_toy_id(), "polytopes");
    }
}
