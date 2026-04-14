//! Static toy registry
//!
//! Creates all known toy instances and provides a stable display order.
//! Each toy's `id()` and `name()` are its canonical source for metadata.

use std::collections::HashMap;

use super::Toy;

#[must_use]
pub(crate) fn create_all_toys() -> HashMap<String, Box<dyn Toy>> {
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

/// Returns toy IDs in the preferred display order.
#[must_use]
pub fn toy_id_order() -> Vec<&'static str> {
    vec!["polytopes", "debug_scratchpad"]
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
    fn test_toy_id_order_matches_registry() {
        let toys = create_all_toys();
        for id in toy_id_order() {
            assert!(
                toys.contains_key(id),
                "ID order references unknown toy: {id}"
            );
        }
    }

    #[test]
    fn test_toy_id_order_includes_all() {
        let toys = create_all_toys();
        let ordered: std::collections::HashSet<&str> = toy_id_order().into_iter().collect();
        for id in toys.keys() {
            assert!(
                ordered.contains(id.as_str()),
                "Toy {id} missing from id_order"
            );
        }
    }
}
