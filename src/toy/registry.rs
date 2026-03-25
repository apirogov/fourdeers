//! Static toy registry

use std::collections::HashMap;

use super::Toy;

pub fn create_all_toys() -> HashMap<String, Box<dyn Toy>> {
    let mut toys = HashMap::new();

    let tesseract = crate::toys::tesseract::TesseractToy::new();
    toys.insert(
        tesseract.id().to_string(),
        Box::new(tesseract) as Box<dyn Toy>,
    );

    toys
}

pub fn toy_ids() -> Vec<&'static str> {
    vec!["tesseract"]
}

pub fn toy_name_by_id(id: &str) -> Option<&'static str> {
    match id {
        "tesseract" => Some("4D Tesseract"),
        _ => None,
    }
}
