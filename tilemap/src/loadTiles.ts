import { TileBounds } from './TileBounds.ts'
import { TileNo, TileSet } from 'tiles-math'

type TileTuple = [number, number] // Tile [x,y] as delivered by the REST endpoint
type TileArray = Array<TileTuple>

export async function loadTiles(tilesUrl: string, bounds: TileBounds, zoom: number, signal: AbortSignal): Promise<TileSet> {
    const params = `bounds=${bounds.x1},${bounds.y1},${bounds.x2},${bounds.y2}`
    const fullUrl = `${tilesUrl}/${zoom}?${params}`
    // console.log('-----> Load tiles', fullUrl)
    const tileSet = new TileSet(zoom)
    try {
        const response = await fetch(fullUrl, { signal })
        const tileArray: TileArray = await response.json()
        tileSet.addTiles(tileArray.map(([x, y]: TileTuple): TileNo => ({x, y})))
    } catch (e) {
        if (e instanceof Error && e.name === 'AbortError') {
            console.log('Request aborted:', fullUrl)
        } else {
            console.warn('Cannot fetch data from server:', e)
        }
    }
    return tileSet
}

