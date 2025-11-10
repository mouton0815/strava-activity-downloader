import { Polyline } from 'react-leaflet'
import { usePositionTracker } from './usePositionTracker.ts'

/**
 * Tracks GPS positions and visualizes them as a polyline on the map.
 * Positions are recorded with a minimum 10-meter distance threshold.
 */
export function GPSTrackPolyline() {
    const positions = usePositionTracker()

    if (positions.length === 0) {
        return null
    }

    return (
        <Polyline
            positions={positions}
            pathOptions={{
                color: 'red',
                weight: 2,
                opacity: 0.7
            }}
        />
    )
}
