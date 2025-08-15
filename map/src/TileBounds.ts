import { coords2tile, TileNo } from 'tiles-math'
import { LatLngBounds } from 'leaflet'

export class TileBounds {
    x1: number
    y1: number
    x2: number
    y2: number
    constructor(upperLeft: TileNo, lowerRight: TileNo) {
        this.x1 = upperLeft.x
        this.y1 = upperLeft.y
        this.x2 = lowerRight.x
        this.y2 = lowerRight.y
    }
    static fromLatLngBounds(bounds: LatLngBounds, zoom: number): TileBounds {
        return new TileBounds(
            coords2tile([bounds.getNorth(), bounds.getWest()], zoom),
            coords2tile([bounds.getSouth(), bounds.getEast()], zoom)
        )
    }
    contains(that: TileBounds): boolean {
        return this.x1 <= that.x1 && this.y1 <= that.y1 && this.x2 >= that.x2 && this.y2 >= that.y2
    }
}

export class TileBoundsMap {
    map: Map<number, TileBounds>
    constructor(map: Map<number, TileBounds> | null = null) {
        this.map = map || new Map<number, TileBounds>()
    }
    static fromLatLngBounds(bounds: LatLngBounds, zoomLevels: Array<number>): TileBoundsMap {
        const map = new Map<number, TileBounds>()
        for (const zoom of zoomLevels) {
            map.set(zoom, TileBounds.fromLatLngBounds(bounds, zoom))
        }
        return new TileBoundsMap(map)
    }
    shallowCopy(): TileBoundsMap {
        return new TileBoundsMap(this.map)
    }
    get(zoom: number): TileBounds {
        const bounds = this.map.get(zoom)
        if (!bounds) {
            throw new Error(`Zoom level ${zoom} not available in map (this is a bug`)
        }
        return bounds
    }
    set(zoom: number, bounds: TileBounds) {
        this.map.set(zoom, bounds)
    }
    contains(bounds: TileBounds, zoom: number): boolean {
        const thisBounds = this.map.get(zoom)
        return !!thisBounds && thisBounds.contains(bounds)
    }
}
