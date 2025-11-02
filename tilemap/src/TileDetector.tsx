import { useEffect } from 'react'
import { coords2tile } from 'tiles-math'
import { useLocationStore } from './useLocationStore.ts'
import { useCandidateStore } from './useCandidateStore.ts'
import { useTileStore } from './useTileStore.ts'

type TileDetectorProps = {
    zoomLevels: Array<number>
}

/**
 * A non-UI component that listens to location changes detects whether the location
 * would lead to a new tile per zoom level. Puts the candidate tiles into a global Zustand.
 */
export const TileDetector = ({ zoomLevels }: TileDetectorProps): null => {
    const position  = useLocationStore(state => state.position)
    const tileStore = useTileStore(state => state.tileStore)
    const { candStore, setCandStore } = useCandidateStore()

    useEffect(() => {
        if (position) {
            let changed = false
            for (const zoom of zoomLevels) {
                const candSet = candStore.get(zoom)
                const tileSet = tileStore.get(zoom)
                // Remove all tiles from the candidate set that are part of the tile set fetched from server.
                // The candidate set may contain regular tiles if the loading of tiles from the server took longer.
                for (const tileNo of candSet) {
                    if (tileSet.has(tileNo)) {
                        candSet.removeTile(tileNo)
                        changed = true
                    }
                }
                // Check if the tile at the current GPS position is among the regular tiles.
                // If not, add it to the set of candidate tiles (for this zoom level).
                const tileNo = coords2tile([position.lat, position.lng], zoom)
                if (!candSet.has(tileNo) && !tileSet.has(tileNo)) {
                    candSet.addTile(tileNo)
                    changed = true
                }

            }
            if (changed) {
                setCandStore(candStore.shallowCopy())
            }
        }
    }, [position, tileStore])

    return null
}