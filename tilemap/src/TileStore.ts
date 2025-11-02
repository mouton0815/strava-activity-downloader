import { TileSet } from 'tiles-math'

/**
 * Holds a {@link TileSet} per zoom level.
 */
export class TileStore {
    map: Map<number, TileSet>
    constructor(map: Map<number, TileSet> | null = null) {
        this.map = map || new Map<number, TileSet>()
    }
    shallowCopy(): TileStore {
        return new TileStore(this.map)
    }
    [Symbol.iterator](): IterableIterator<TileSet> {
        return this.map.values()[Symbol.iterator]();
    }
    get(zoom: number): TileSet {
        if (!this.map.has(zoom)) {
            this.map.set(zoom, new TileSet(zoom))
        }
        return this.map.get(zoom)!
    }
    set(zoom: number, tiles: TileSet): TileStore {
        this.map.set(zoom, tiles)
        return this
    }
}