//! Static toy registry

use std::collections::HashMap;

use super::Toy;

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

pub fn toy_ids() -> Vec<&'static str> {
    vec!["polytopes", "debug_scratchpad"]
}

pub fn toy_name_by_id(id: &str) -> Option<&'static str> {
    match id {
        "polytopes" => Some("Polytopes"),
        "debug_scratchpad" => Some("DebugScratchpad"),
        _ => None,
    }
}
