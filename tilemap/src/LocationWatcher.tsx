import { useEffect } from 'react'
import { ErrorEvent, LocationEvent } from 'leaflet'
import { useMap } from 'react-leaflet'
import { useLocationStore } from './useLocationStore.ts'

/**
 * A non-UI component that watches to location changes and puts the location into a global Zustand.
 */
export function LocationWatcher(): null {
    const setPosition = useLocationStore(state => state.setPosition)
    const map = useMap()

    useEffect(() => {
        const handleLocationFound = (event: LocationEvent) => {
            setPosition(event.latlng)
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
    }, [map, setPosition])

    return null
}
