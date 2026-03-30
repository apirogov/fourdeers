//! Toy manager for switching between toys

use std::collections::HashMap;

use super::Toy;

pub struct ToyManager {
    toys: HashMap<String, Box<dyn Toy>>,
    active_toy_id: String,
}

impl ToyManager {
    pub fn new() -> Self {
        let toys = super::registry::create_all_toys();
        let active_toy_id = "polytopes".to_string();

        Self {
            toys,
            active_toy_id,
        }
    }

    pub fn active_toy(&self) -> &dyn Toy {
        self.toys
            .get(&self.active_toy_id)
            .map(|t| t.as_ref())
            .expect("Active toy should always exist")
    }

    pub fn active_toy_mut(&mut self) -> &mut dyn Toy {
        self.toys
            .get_mut(&self.active_toy_id)
            .map(|t| t.as_mut())
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

    pub fn toy_list(&self) -> Vec<(&str, &str)> {
        super::registry::toy_ids()
            .into_iter()
            .filter_map(|id| {
                let name = super::registry::toy_name_by_id(id)?;
                Some((id, name))
            })
            .collect()
    }

    pub fn active_toy_id(&self) -> &str {
        &self.active_toy_id
    }

    pub fn active_toy_name(&self) -> &str {
        self.active_toy().name()
    }
}

impl Default for ToyManager {
    fn default() -> Self {
        Self::new()
    }
}
