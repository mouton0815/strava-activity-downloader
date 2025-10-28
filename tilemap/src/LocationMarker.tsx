import { divIcon } from 'leaflet'
import { Marker, useMap } from 'react-leaflet'
import { useLocationStore } from './useLocationStore.ts'

type LocationMarkerProps = {
    crossHairSize: number
}

/**
 * Puts a crosshair marker on the map for the current GPS location
 * and pans the map to the new location (keeping the zoom level).
 * The location is delivered by {@link LocationWatcher}.
 */
export function LocationMarker({ crossHairSize }: LocationMarkerProps) {
    const position = useLocationStore(state => state.position)
    const map = useMap()

    const icon = divIcon({
        className: 'crosshair-marker',
        iconSize: [crossHairSize, crossHairSize],
        iconAnchor: [crossHairSize / 2, crossHairSize / 2] // Centered
    })

    if (!position) return null

    map.panTo(position) // Pan but keep current zoom

    return <Marker position={position} icon={icon} />
}
