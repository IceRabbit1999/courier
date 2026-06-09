/// A hero's name in a single locale, as fetched from one API call. The sync layer
/// upserts one of these per (hero, locale) pair into the normalized name table.
#[derive(Debug, Clone)]
pub struct HeroEntry {
    pub id: i32,
    pub internal_name: String,
    pub display_name: String,
}
