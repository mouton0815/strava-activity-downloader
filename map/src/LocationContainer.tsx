import { useEffect, useState } from 'react'
import { divIcon, LatLng, marker } from 'leaflet'
import { useMapEvents } from 'react-leaflet'

type LocationContainerProps = {
    crossHairSize: number
}

/** Puts a crosshair marker on the map for the current GPS location */
export function LocationContainer({ crossHairSize }: LocationContainerProps) {
    const [location, setLocation] = useState<LatLng | null>(null)
    const map = useMapEvents({
        locationfound: (event) => {
            const icon = divIcon({
                className: 'crosshair-marker',
                iconSize: [crossHairSize, crossHairSize],
                iconAnchor: [crossHairSize / 2, crossHairSize / 2] // Centered
            })
            marker(event.latlng, { icon }).addTo(map);
        },
        locationerror: (event) => {
            console.warn(event.message)
            alert(event.message)
        }
    })

    useEffect(() => {
        if (location === null) {
            map.locate({setView: true, maxZoom: 14})
            setLocation(map.getCenter())
        }
    }, [location])

    return <div />
}
