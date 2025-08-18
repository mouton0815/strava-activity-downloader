type TileTuple = [number, number] // Tile [x,y] as delivered by the REST endpoint
export type TileArray = Array<TileTuple>

export class TileCache {
    map: Map<number, TileArray> // zoom -> [tile, tile, ...]
    constructor(map: Map<number, TileArray> | null = null) {
        this.map = map || new Map<number, TileArray>()
    }
    shallowCopy(): TileCache {
        return new TileCache(this.map)
    }
    [Symbol.iterator](): IterableIterator<[number, TileArray]> {
        return this.map[Symbol.iterator]();
    }
    get(key: number): TileArray | undefined {
        return this.map.get(key)
    }
    set(key: number, value: TileArray): TileCache {
        this.map.set(key, value)
        return this
    }
}