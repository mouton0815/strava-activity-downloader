import { useEffect, useState } from 'react'
import { useMapEvents } from 'react-leaflet'
import { TileNo, TileSet } from 'tiles-math'
import { TileBounds, TileBoundsMap } from '../types/TileBounds.ts'
import { TileStore } from '../types/TileStore.ts'

/**
 * A hook that watches map zooms and pans and loads the tiles for the resulting
 * bounding box from the server.
 */
export function useFetchedTiles(tilesUrl: string, zoomLevels: Array<number>): TileStore {
    const [newBounds, setNewBounds] = useState<TileBoundsMap | null>(null)
    const [maxBounds, setMaxBounds] = useState<TileBoundsMap>(new TileBoundsMap())
    const [fetchedTiles, setFetchedTiles] = useState<TileStore>(new TileStore())

    // React on map events (to determine location and to determine visible map bounds for tile loading)
    const map = useMapEvents({
        moveend: () => {
            setNewBounds(TileBoundsMap.fromLatLngBounds(map.getBounds(), zoomLevels))
        },
        zoomend: () => {
            setNewBounds(TileBoundsMap.fromLatLngBounds(map.getBounds(), zoomLevels))
        }
    })

    // The 'load' event seems to be fired too early, so listen to the whenReady callback for the initial load
    useEffect(() => {
        map.whenReady(() => {
            //console.log('-----> ready')
            setNewBounds(TileBoundsMap.fromLatLngBounds(map.getBounds(), zoomLevels))
        })
    }, [map, zoomLevels])

    // Load (and cache) tiles according to the visible map bounds
    useEffect(() => {
        const controller = new AbortController()
        let isCancelled = false

        async function fetchData() {
            if (newBounds) { // Null until the initial Leaflet event
                let hasChanged = false
                for (const zoom of zoomLevels) {
                    const bounds = newBounds.get(zoom)
                    if (!maxBounds.contains(bounds, zoom)) {
                        const tiles = await fetchTiles(tilesUrl, bounds, zoom, controller.signal)
                        if (!isCancelled) {
                            fetchedTiles.set(zoom, tiles)
                            maxBounds.set(zoom, bounds)
                            hasChanged = true
                        }
                    }
                }
                if (hasChanged && !isCancelled) {
                    setFetchedTiles(fetchedTiles.shallowCopy()) // Shallow copy
                    setMaxBounds(maxBounds.shallowCopy())
                }
            }
        }

        fetchData()

        // The cleanup functions makes sure that the current tile fetching request is cancelled
        // if newBounds has changed in the meantime. This happens during panning or zooming.
        // Note that in theory, the change of any useEffect parameters could trigger this effect,
        // but in reality only newBounds can change independently of the useEffect function.
        return () => {
            controller.abort()
            isCancelled = true;
        }
    }, [maxBounds, newBounds, setFetchedTiles, fetchedTiles, tilesUrl, zoomLevels])

    return fetchedTiles
}

type TileTuple = [number, number] // Tile [x,y] as delivered by the REST endpoint
type TileArray = Array<TileTuple>

async function fetchTiles(tilesUrl: string, bounds: TileBounds, zoom: number, signal: AbortSignal): Promise<TileSet> {
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
