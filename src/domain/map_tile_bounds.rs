#[derive(Debug)]
pub struct MapTileBounds {
    pub x1: u64,
    pub y1: u64,
    pub x2: u64,
    pub y2: u64
}

impl MapTileBounds {
    pub fn new(x1: u64, y1: u64, x2: u64, y2: u64) -> Self {
        Self { x1, y1, x2, y2 }
    }
}