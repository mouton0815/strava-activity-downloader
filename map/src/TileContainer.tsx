import { useEffect, useState } from 'react'
import { TileBoundsMap } from './TileBounds.ts'
import { TileCache } from './TileCache.ts'
import { useMapEvents } from 'react-leaflet'
import { TileOverlays } from './TileOverlays.tsx'
import { loadTiles } from './loadTiles.ts'

type TileContainerProps = {
    tilesUrl: string
    zoomLevels: Array<number>
    tileColors: Array<string>
}

export function TileContainer({ tilesUrl, zoomLevels, tileColors }: TileContainerProps) {
    const [newBounds, setNewBounds] = useState<TileBoundsMap | null>(null)
    const [maxBounds, setMaxBounds] = useState<TileBoundsMap>(new TileBoundsMap())
    const [tileCache, setTileCache] = useState<TileCache>(new TileCache())

    // React on map events (to determine location and to determine visible map bounds for tile loading)
    const map = useMapEvents({
        moveend: () => {
            //console.log('-----> moved')
            setNewBounds(TileBoundsMap.fromLatLngBounds(map.getBounds(), zoomLevels))
        },
        zoomend: () => {
            //console.log('-----> zoomed')
            setNewBounds(TileBoundsMap.fromLatLngBounds(map.getBounds(), zoomLevels))
        }
    })

    // The 'load' event seems to be fired too early, so listen to the whenReady callback for the initial load
    useEffect(() => {
        map.whenReady(() => {
            //console.log('-----> ready')
            setNewBounds(TileBoundsMap.fromLatLngBounds(map.getBounds(), zoomLevels))
        })
    }, [map])

    // Load (and cache) tiles according to the visible map bounds
    useEffect(() => {
        let isCancelled = false

        async function fetchData() {
            if (newBounds) { // Null until the initial Leaflet event
                for (const zoom of zoomLevels) {
                    const bounds = newBounds.get(zoom)
                    if (!maxBounds.contains(bounds, zoom)) {
                        const tiles = await loadTiles(tilesUrl, bounds, zoom)
                        if (!isCancelled) {
                            tileCache.set(zoom, tiles)
                            maxBounds.set(zoom, bounds)
                        }
                    }
                }
                if (!isCancelled) {
                    setTileCache(tileCache.shallowCopy()) // Shallow copy
                    setMaxBounds(maxBounds.shallowCopy())
                }
            }
        }

        fetchData()

        return () => {
            isCancelled = true;
        }
    }, [newBounds])

    // console.log("-----> PASS with ", bounds)
    return <TileOverlays tileCache={tileCache} tileColors={tileColors} />
}
