use std::f64::consts::PI;
use crate::domain::tile::Tile;

/// Calculates the x,y part of a tile name (see https://wiki.openstreetmap.org/wiki/Slippy_map_tilenames)
/// from a latitude-longitude pair plus zoom level, and creates a [Tile] object.
/// @param lat - a latitude
/// @param lon - a longitude
/// @param zoom - a map zoom level
/// @returns the corresponding tile number
pub fn coords2tile(lat: f64, lon: f64, zoom: u16) -> Tile {
    let z_pow = (1 << zoom) as f64; // Math.pow(2, zoom)
    let lat_rad = (lat * PI) / 180.0;
    let x = (((lon + 180.0) / 360.0) * z_pow).floor() as u64;
    let y = (((1.0 - (lat_rad.tan() + 1.0 / lat_rad.cos()).ln() / PI) / 2.0) * z_pow).floor() as u64;
    Tile { x, y }
}

#[cfg(test)]
mod tests {
    use crate::util::coords2tile::{coords2tile, Tile};

    const ZOOM: u16 = 14;

    // Jena city center tile inner coord and edge coords
    const JENA_LAT_N: f64 = 50.930738023718185;
    const JENA_LAT_S: f64 = 50.91688748924508; // should be ...4507 but that might hit the precision limits
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
        let tile = coords2tile((JENA_LAT_N + JENA_LAT_S) / 2.0, (JENA_LON_W + JENA_LON_E) / 2.0, ZOOM);
        assert_eq!(tile, Tile { x: JENA_X, y: JENA_Y });
    }

    #[test]
    fn test_jena_c_nw() {
        let tile = coords2tile(JENA_LAT_N, JENA_LON_W, ZOOM);
        assert_eq!(tile, Tile { x: JENA_X, y: JENA_Y });
    }

    #[test]
    fn test_jena_c_ne() {
        let tile = coords2tile(JENA_LAT_N, JENA_LON_E, ZOOM);
        assert_eq!(tile, Tile { x: JENA_X, y: JENA_Y });
    }

    #[test]
    fn test_jena_c_sw() {
        let tile = coords2tile(JENA_LAT_S, JENA_LON_W, ZOOM);
        assert_eq!(tile, Tile { x: JENA_X, y: JENA_Y });
    }

    #[test]
    fn test_jena_c_se() {
        let tile = coords2tile(JENA_LAT_S, JENA_LON_E, ZOOM);
        assert_eq!(tile, Tile { x: JENA_X, y: JENA_Y });
    }

    // Jena tiles around center tile
    #[test]
    fn test_jena_n_sw() {
        let tile = coords2tile(JENA_LAT_N + DELTA, JENA_LON_W, ZOOM);
        assert_eq!(tile, Tile { x: JENA_X, y: JENA_Y - 1 });
    }

    #[test]
    fn test_jena_w_ne() {
        let tile = coords2tile(JENA_LAT_N, JENA_LON_W - DELTA, ZOOM);
        assert_eq!(tile, Tile { x: JENA_X - 1, y: JENA_Y });
    }

    #[test]
    fn test_jena_s_nw() {
        let tile = coords2tile(JENA_LAT_S - DELTA, JENA_LON_W, ZOOM);
        assert_eq!(tile, Tile { x: JENA_X, y: JENA_Y + 1 });
    }

    #[test]
    fn test_jena_e_nw() {
        let tile = coords2tile(JENA_LAT_N, JENA_LON_E + DELTA, ZOOM);
        assert_eq!(tile, Tile { x: JENA_X + 1, y: JENA_Y });
    }

    // Zero coordinate
    #[test]
    fn test_zero() {
        let tile = coords2tile(0.0, 0.0, ZOOM);
        assert_eq!(tile, Tile { x: ZERO_X, y: ZERO_Y });
    }
}