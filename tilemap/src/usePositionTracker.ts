import { useEffect, useState } from 'react'
import { LatLngTuple } from 'leaflet'
import haversine from 'haversine-distance'
import { useLocation } from './useLocation.ts'

const DISTANCE_THRESHOLD = 10 // meters

/**
 * A hook that tracks all GPS positions with a minimum distance threshold of 10 meters
 * between consecutive recorded positions. Returns an array of recorded positions
 * compatible with Leaflet polylines.
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
                    return [...prevPositions, [location.lat, location.lng]]
                }

                return prevPositions
            })
        }
    }, [location])

    return positions
}
