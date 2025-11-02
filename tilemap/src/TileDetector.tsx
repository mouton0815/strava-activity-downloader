import { useEffect } from 'react'
import { coords2tile } from 'tiles-math'
import { useLocationStore } from './useLocationStore.ts'
import { useDetectedTiles } from './useDetectedTiles.ts'
import { useFetchedTiles } from './useFetchedTiles.ts'

type TileDetectorProps = {
    zoomLevels: Array<number>
}

/**
 * A non-UI component that listens to location changes detects whether the location
 * would lead to a new tile per zoom level. Puts the detected tiles into a global Zustand.
 */
export const TileDetector = ({ zoomLevels }: TileDetectorProps): null => {
    const position  = useLocationStore(state => state.position)
    const fetchedTiles = useFetchedTiles(state => state.fetchedTiles)
    const { detectedTiles, setDetectedTiles } = useDetectedTiles()

    useEffect(() => {
        if (position) {
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
                const tileNo = coords2tile([position.lat, position.lng], zoom)
                if (!detectedSet.has(tileNo) && !fetchedSet.has(tileNo)) {
                    detectedSet.addTile(tileNo)
                    changed = true
                }

            }
            if (changed) {
                setDetectedTiles(detectedTiles.shallowCopy())
            }
        }
    }, [detectedTiles, position, setDetectedTiles, fetchedTiles, zoomLevels])

    return null
}