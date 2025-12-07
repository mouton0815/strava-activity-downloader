import { useEffect } from 'react'
import { Marker, useMap } from 'react-leaflet'
import { divIcon } from 'leaflet'
import { useLocation } from '../hooks/useLocation.ts'

type LocationMarkerProps = {
    crossHairSize: number
}

/**
 * Puts a crosshair marker on the map for the current GPS location
 * and pans the map to the new location (keeping the zoom level).
 * The location is delivered by the {@link useLocation} hook.
 */
export function LocationMarker({ crossHairSize }: LocationMarkerProps) {
    const location = useLocation()
    const map = useMap()

    const icon = divIcon({
        className: 'crosshair-marker',
        iconSize: [crossHairSize, crossHairSize],
        iconAnchor: [crossHairSize / 2, crossHairSize / 2] // Centered
    })

    useEffect(() => {
        if (location) {
            map.panTo(location) // Pan but keep current zoom
        }
    }, [map, location])

    if (!location) return null

    return <Marker position={location} icon={icon} />
}
