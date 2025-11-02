import { useEffect, useState } from 'react'
import { ErrorEvent, LatLng, LocationEvent } from 'leaflet'
import { useMap } from 'react-leaflet'

/**
 * A hook that watches changes of the GPS location.
 */
export function useLocation(): LatLng | null {
    const [location, setLocation] = useState<LatLng | null>(null)
    const map = useMap()

    useEffect(() => {
        const handleLocationFound = (event: LocationEvent) => {
            setLocation(event.latlng)
        }

        const handleLocationError = (event: ErrorEvent) => {
            console.warn(event.message)
        }

        map.on("locationfound", handleLocationFound)
        map.on("locationerror", handleLocationError)
        map.locate({ enableHighAccuracy: true, watch: true })

        return () => {
            map.off("locationfound", handleLocationFound)
            map.off("locationerror", handleLocationError)
            map.stopLocate()
        }
    }, [map, setLocation])

    return location
}
