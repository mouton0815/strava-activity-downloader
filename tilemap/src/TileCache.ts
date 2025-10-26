import { TileSet } from 'tiles-math'

export class TileCache {
    map: Map<number, TileSet> // zoom -> [tile, tile, ...]
    constructor(map: Map<number, TileSet> | null = null) {
        this.map = map || new Map<number, TileSet>()
    }
    shallowCopy(): TileCache {
        return new TileCache(this.map)
    }
    [Symbol.iterator](): IterableIterator<TileSet> {
        return this.map.values()[Symbol.iterator]();
    }
    get(zoom: number): TileSet | undefined {
        return this.map.get(zoom)
    }
    set(zoom: number, tiles: TileSet): TileCache {
        this.map.set(zoom, tiles)
        return this
    }
}