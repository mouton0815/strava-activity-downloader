/// Supported slippy-map zoom levels, see https://wiki.openstreetmap.org/wiki/Zoom_levels.
/// Note that the tile algorithms work with any zoom level, but the database is restricted
/// to the levels in this enum.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum MapZoom {
    Level14,
    Level17
}

impl MapZoom {
    pub const VALUES: [Self; 2] = [Self::Level14, Self::Level17];

    pub fn value(&self) -> u16 {
        match *self {
            MapZoom::Level14 => 14,
            MapZoom::Level17 => 17
        }
    }
}