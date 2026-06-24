/// An item's name in a single locale, as fetched from one API call. The sync layer
/// upserts one of these per (item, locale) pair into the normalized name table.
#[derive(Debug, Clone)]
pub struct ItemEntry {
    pub id: i32,
    pub short_name: String,
    pub display_name: String,
}
