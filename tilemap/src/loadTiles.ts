import { TileBounds } from './TileBounds.ts'
import { TileArray } from './TileCache.ts'

export async function loadTiles(tilesUrl: string, bounds: TileBounds, zoom: number, signal: AbortSignal): Promise<TileArray> {
    const params = `bounds=${bounds.x1},${bounds.y1},${bounds.x2},${bounds.y2}`
    const fullUrl = `${tilesUrl}/${zoom}?${params}`
    // console.log('-----> Load tiles', fullUrl)
    try {
        const response = await fetch(fullUrl, { signal })
        return await response.json()
    } catch (e) {
        if (e instanceof Error && e.name === 'AbortError') {
            console.log('Request aborted:', fullUrl)
        } else {
            console.warn('Cannot fetch data from server:', e)
        }
        return []
    }
}

