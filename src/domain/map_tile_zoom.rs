#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum MapTileZoom {
    ZOOM14,
    ZOOM17
}

impl MapTileZoom {
    pub const VALUES: [Self; 2] = [Self::ZOOM14, Self::ZOOM17];

    pub fn value(&self) -> u16 {
        match *self {
            MapTileZoom::ZOOM14 => 14,
            MapTileZoom::ZOOM17 => 17
        }
    }
}