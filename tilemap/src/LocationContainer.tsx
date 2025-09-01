import { useEffect, useState } from 'react'
import { divIcon, ErrorEvent, LatLng, LocationEvent } from 'leaflet'
import { Marker, useMap } from 'react-leaflet'

type LocationContainerProps = {
    crossHairSize: number
}

/** Puts a crosshair marker on the map for the current GPS location */
export function LocationContainer({ crossHairSize }: LocationContainerProps) {
    const [position, setPosition] = useState<LatLng | null>(null)
    const map = useMap()

    const icon = divIcon({
        className: 'crosshair-marker',
        iconSize: [crossHairSize, crossHairSize],
        iconAnchor: [crossHairSize / 2, crossHairSize / 2] // Centered
    })

    useEffect(() => {
        const handleLocationFound = (event: LocationEvent) => {
            // console.log('-----> Location event', event.latlng)
            map.panTo(event.latlng) // Pan but keep current zoom
            setPosition(event.latlng)
        }

        const handleLocationError = (event: ErrorEvent) => {
            console.warn(event.message)
        }

        map.on("locationfound", handleLocationFound)
        map.on("locationerror", handleLocationError)
        map.locate({ watch: true })

        return () => {
            map.off("locationfound", handleLocationFound)
            map.off("locationerror", handleLocationError)
            map.stopLocate()
        }
    }, [map])

    if (!position) return null

    return <Marker position={position} icon={icon} />
}
