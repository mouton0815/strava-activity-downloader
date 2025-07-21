use std::f64::consts::PI;
use serde::Serialize;
use crate::domain::map_zoom::MapZoom;

/// Represents a slippy map tile (see https://wiki.openstreetmap.org/wiki/Slippy_map_tilenames)
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct MapTile(u64, u64);

impl MapTile {
    pub fn new(x: u64, y: u64) -> Self {
        Self(x, y)
    }

    /// Calculates the x,y part of a tile name from a latitude-longitude pair plus zoom level,
    /// and creates a [MapTile] object.
    /// @param lat - a latitude
    /// @param lon - a longitude
    /// @param zoom - a map zoom level
    /// @returns the corresponding tile number
    pub fn from_coords(lat: f64, lon: f64, zoom: MapZoom) -> Self {
        let z_pow = (1 << zoom.value()) as f64; // Math.pow(2, zoom)
        let lat_rad = (lat * PI) / 180.0;
        let x = (((lon + 180.0) / 360.0) * z_pow).floor() as u64;
        let y = (((1.0 - (lat_rad.tan() + 1.0 / lat_rad.cos()).ln() / PI) / 2.0) * z_pow).floor() as u64;
        Self(x, y)
    }

    pub fn get_x(&self) -> u64 {
        self.0
    }

    pub fn get_y(&self) -> u64 {
        self.1
    }
}

#[cfg(test)]
mod tests {
    use crate::domain::map_tile::MapTile;
    use crate::domain::map_zoom::MapZoom;

    const ZOOM: MapZoom = MapZoom::Level14;

    // Jena city center tile inner coord and edge coords
    // Collected with help of https://chrishewett.com/blog/slippy-tile-explorer/?
    const JENA_LAT_N: f64 = 50.930738023718185;
    const JENA_LAT_S: f64 = 50.91688748924508;
    // should be ...4507 but that might hit the precision limits
    const JENA_LON_W: f64 = 11.57958984375;
    const JENA_LON_E: f64 = 11.6015624999999;
    const DELTA: f64 = 0.000000000001;
    const JENA_X: u64 = 8719;
    const JENA_Y: u64 = 5490;
    // Zero coordinate
    const ZERO_X: u64 = 8192;
    const ZERO_Y: u64 = 8192;

    #[test]
    fn test_jena_c_inner() {
        let tile = MapTile::from_coords((JENA_LAT_N + JENA_LAT_S) / 2.0, (JENA_LON_W + JENA_LON_E) / 2.0, ZOOM);
        assert_eq!(tile, MapTile::new(JENA_X, JENA_Y));
    }


    #[test]
    fn test_jena_c_nw() {
        let tile = MapTile::from_coords(JENA_LAT_N, JENA_LON_W, ZOOM);
        assert_eq!(tile, MapTile::new(JENA_X, JENA_Y));
    }

    #[test]
    fn test_jena_c_ne() {
        let tile = MapTile::from_coords(JENA_LAT_N, JENA_LON_E, ZOOM);
        assert_eq!(tile, MapTile::new(JENA_X, JENA_Y));
    }

    #[test]
    fn test_jena_c_sw() {
        let tile = MapTile::from_coords(JENA_LAT_S, JENA_LON_W, ZOOM);
        assert_eq!(tile, MapTile::new(JENA_X, JENA_Y));
    }

    #[test]
    fn test_jena_c_se() {
        let tile = MapTile::from_coords(JENA_LAT_S, JENA_LON_E, ZOOM);
        assert_eq!(tile, MapTile::new(JENA_X, JENA_Y));
    }

    // Jena tiles around center tile
    #[test]
    fn test_jena_n_sw() {
        let tile = MapTile::from_coords(JENA_LAT_N + DELTA, JENA_LON_W, ZOOM);
        assert_eq!(tile, MapTile::new(JENA_X, JENA_Y - 1));
    }

    #[test]
    fn test_jena_w_ne() {
        let tile = MapTile::from_coords(JENA_LAT_N, JENA_LON_W - DELTA, ZOOM);
        assert_eq!(tile, MapTile::new(JENA_X - 1, JENA_Y));
    }

    #[test]
    fn test_jena_s_nw() {
        let tile = MapTile::from_coords(JENA_LAT_S - DELTA, JENA_LON_W, ZOOM);
        assert_eq!(tile, MapTile::new(JENA_X, JENA_Y + 1));
    }

    #[test]
    fn test_jena_e_nw() {
        let tile = MapTile::from_coords(JENA_LAT_N, JENA_LON_E + DELTA, ZOOM);
        assert_eq!(tile, MapTile::new(JENA_X + 1, JENA_Y));
    }

    // Zero coordinate
    #[test]
    fn test_zero() {
        let tile = MapTile::from_coords(0.0, 0.0, ZOOM);
        assert_eq!(tile, MapTile::new(ZERO_X, ZERO_Y));
    }

    /*
    #[test]
    fn test_temp() {
        let tile = MapTile::from_coords(1.0, 1.0, ZOOM); // (8237, 8146)
        let tile = MapTile::from_coords(2.0, 2.0, ZOOM); // (8283, 8100)
        let tile = MapTile::from_coords(3.0, 3.0, ZOOM); // (8328, 8055)
        assert_eq!(tile, MapTile::new(8328, 8055));
    }
    */
}