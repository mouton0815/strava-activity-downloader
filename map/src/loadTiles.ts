import { TileBounds } from './TileBounds.ts'
import { TileArray } from './TileCache.ts'

export async function loadTiles(tilesUrl: string, bounds: TileBounds, zoom: number): Promise<TileArray> {
    try {
        const boundsParam = `bounds=${bounds.x1 + 2},${bounds.y1 + 2},${bounds.x2 - 2},${bounds.y2 - 2}`
        const response = await fetch(`${tilesUrl}/${zoom}?${boundsParam}`)
        return await response.json()
    } catch (e) {
        console.warn('Cannot fetch data from server:', e)
        return []
    }
}

