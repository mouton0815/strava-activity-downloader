import { useEffect, useState } from 'react'
import { useMapEvents } from 'react-leaflet'
import { TileBoundsMap } from './TileBounds.ts'
import { loadTiles } from './loadTiles.ts'
import { useTileStore } from './useTileStore.ts'

type TileFetcherProps = {
    tilesUrl: string
    zoomLevels: Array<number>
}

/**
 * A non-UI component that watches map zooms and pans and loads the tiles for the resulting
 * bounding box from the server. Then it puts the tiles into a global Zustand.
 */
export function TileFetcher({ tilesUrl, zoomLevels }: TileFetcherProps): null {
    const [newBounds, setNewBounds] = useState<TileBoundsMap | null>(null)
    const [maxBounds, setMaxBounds] = useState<TileBoundsMap>(new TileBoundsMap())
    const { tileStore, setTileStore } = useTileStore()

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
                        const tiles = await loadTiles(tilesUrl, bounds, zoom, controller.signal)
                        if (!isCancelled) {
                            tileStore.set(zoom, tiles)
                            maxBounds.set(zoom, bounds)
                            hasChanged = true
                        }
                    }
                }
                if (hasChanged && !isCancelled) {
                    setTileStore(tileStore.shallowCopy()) // Shallow copy
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
    }, [maxBounds, newBounds, setTileStore, tileStore, tilesUrl, zoomLevels])

    return null
}

