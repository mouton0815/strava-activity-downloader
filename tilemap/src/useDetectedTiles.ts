import { TileStore } from './TileStore.ts'
import { useEffect, useState } from 'react'
import { coords2tile } from 'tiles-math'
import { useLocation } from './useLocation.ts'

/**
 * A hook that checks if the current GPS location would lead to new tiles for the given zoom levels.
 * It returns the set of detected tiles.
 */
export const useDetectedTiles = (fetchedTiles: TileStore, zoomLevels: Array<number>): TileStore => {
    const location  = useLocation()
    const [detectedTiles, setDetectedTiles] = useState<TileStore>(new TileStore())

    useEffect(() => {
        if (location) {
            let changed = false
            for (const zoom of zoomLevels) {
                const detectedSet = detectedTiles.get(zoom)
                const fetchedSet = fetchedTiles.get(zoom)
                // Remove all tiles from the detected set that are part of the tile set fetched from server.
                // The detected set may contain fetched tiles if the loading of tiles from the server took longer.
                for (const tileNo of detectedSet) {
                    if (fetchedSet.has(tileNo)) {
                        detectedSet.removeTile(tileNo)
                        changed = true
                    }
                }
                // Check if the tile at the current GPS position is among the fetched tiles.
                // If not, add it to the set of detected tiles (for this zoom level).
                const tileNo = coords2tile([location.lat, location.lng], zoom)
                if (!detectedSet.has(tileNo) && !fetchedSet.has(tileNo)) {
                    detectedSet.addTile(tileNo)
                    changed = true
                }

            }
            if (changed) {
                setDetectedTiles(detectedTiles.shallowCopy())
            }
        }
    }, [detectedTiles, location, setDetectedTiles, fetchedTiles, zoomLevels])

    return detectedTiles
}