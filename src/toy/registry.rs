//! Static toy registry
//!
//! Creates all known toy instances and provides ID-to-name lookups.
//! Each toy's `id()` and `name()` are its canonical source for the mapping.

use std::collections::HashMap;

use super::Toy;

#[must_use]
pub fn create_all_toys() -> HashMap<String, Box<dyn Toy>> {
    let mut toys = HashMap::new();

    let polytopes = crate::toys::polytopes::PolytopesToy::new();
    toys.insert(
        polytopes.id().to_string(),
        Box::new(polytopes) as Box<dyn Toy>,
    );

    let debug_scratchpad = crate::toys::debug_scratchpad::DebugScratchpadToy::new();
    toys.insert(
        debug_scratchpad.id().to_string(),
        Box::new(debug_scratchpad) as Box<dyn Toy>,
    );

    toys
}

#[must_use]
pub fn toy_ids() -> Vec<&'static str> {
    vec!["polytopes", "debug_scratchpad"]
}

#[must_use]
pub fn toy_name_by_id(id: &str) -> Option<&'static str> {
    match id {
        "polytopes" => Some("Polytopes"),
        "debug_scratchpad" => Some("DebugScratchpad"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_all_toys_creates_known_toys() {
        let toys = create_all_toys();
        assert_eq!(toys.len(), 2);
        assert!(toys.contains_key("polytopes"));
        assert!(toys.contains_key("debug_scratchpad"));
    }

    #[test]
    fn test_toy_ids_returns_known_ids() {
        let ids = toy_ids();
        assert!(ids.contains(&"polytopes"));
        assert!(ids.contains(&"debug_scratchpad"));
    }

    #[test]
    fn test_toy_name_by_id_returns_names() {
        assert_eq!(toy_name_by_id("polytopes"), Some("Polytopes"));
        assert_eq!(toy_name_by_id("debug_scratchpad"), Some("DebugScratchpad"));
        assert_eq!(toy_name_by_id("nonexistent"), None);
    }
}
