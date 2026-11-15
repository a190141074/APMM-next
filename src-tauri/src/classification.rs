//! Classification domain primitives.
//!
//! Kept dependency-light so filename/category rules can move out of `lib.rs`
//! without changing the serialized library format.

pub(crate) fn category_for_section(section: &str) -> &'static str {
    match section {
        "event_card" | "hand_card" | "map_event" => "cards",
        "monster_card" | "monster_bust" => "monsters",
        "temporary" => "temporary",
        _ => "heroes",
    }
}

pub(crate) fn is_supported_image_extension(ext: &str) -> bool {
    matches!(
        ext.to_ascii_lowercase().as_str(),
        "png" | "jpg" | "jpeg" | "webp"
    )
}
