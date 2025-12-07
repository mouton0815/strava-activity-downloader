import { useEffect, useState } from 'react'
import { LatLngTuple } from 'leaflet'
import haversine from 'haversine-distance'
import { useLocation } from './useLocation.ts'

const DISTANCE_THRESHOLD = 10 // meters
const MAX_POSITIONS = 200
const JUMP_DISTANCE_THRESHOLD = 500 // meters

/**
 * A hook that tracks all GPS positions with a minimum distance threshold of 10 meters
 * between consecutive recorded positions. Returns an array of recorded positions
 * compatible with Leaflet polylines.
 * 
 * The positions array is pruned when:
 * 1. It exceeds 200 entries (oldest positions are removed)
 * 2. The distance between the last position and new position exceeds 500 meters
 */
export function usePositionTracker(): LatLngTuple[] {
    const location = useLocation()
    const [positions, setPositions] = useState<LatLngTuple[]>([])

    useEffect(() => {
        if (location) {
            setPositions((prevPositions) => {
                if (prevPositions.length === 0) {
                    return [[location.lat, location.lng]]
                }

                const lastPosition = prevPositions[prevPositions.length - 1]
                const distance = haversine(
                    { latitude: lastPosition[0], longitude: lastPosition[1] },
                    { latitude: location.lat, longitude: location.lng }
                )

                if (distance >= DISTANCE_THRESHOLD) {
                    const newPosition: LatLngTuple = [location.lat, location.lng]
                    let updatedPositions = [...prevPositions, newPosition]
                    // Prune if distance jump exceeds threshold
                    if (distance > JUMP_DISTANCE_THRESHOLD) {
                        updatedPositions = [newPosition]
                    }
                    // Prune if array exceeds max positions, keeping the newest position
                    else if (updatedPositions.length > MAX_POSITIONS) {
                        updatedPositions = updatedPositions.slice(-MAX_POSITIONS)
                    }
                    return updatedPositions
                }

                return prevPositions
            })
        }
    }, [location])

    return positions
}
