import { useEffect, useState } from 'react'
import { divIcon, LatLng, marker } from 'leaflet'
import { useMapEvents } from 'react-leaflet'

const CROSSHAIR_SIZE = 50

export function LocationContainer() {
    const [location, setLocation] = useState<LatLng | null>(null)
    const map = useMapEvents({
        locationfound: (event) => {
            const icon = divIcon({
                className: 'crosshair-marker',
                iconSize: [CROSSHAIR_SIZE, CROSSHAIR_SIZE],
                iconAnchor: [CROSSHAIR_SIZE / 2, CROSSHAIR_SIZE / 2] // Centered
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
