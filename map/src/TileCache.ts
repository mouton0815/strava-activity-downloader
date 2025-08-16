type TileTuple = [number, number] // Tile [x,y] as delivered by the REST endpoint
export type TileArray = Array<TileTuple>
// TODO: Wrap map and expose iterator
export type TileCache = Map<number, TileArray> // zoom -> [tile, tile, ...]

