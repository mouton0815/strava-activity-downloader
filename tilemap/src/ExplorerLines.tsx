import { Polyline, useMapEvents } from 'react-leaflet'
import { useEffect, useState } from 'react'
import { Coords } from 'tiles-math'
import { LatLngBounds } from 'leaflet'

/**
 * Calculates the latitude of a tile given its y position and zoom level
 * @param y - the y coordinate of a tile
 * @param zoom the map zoom level
 * @returns the corresponding latitude
 */
export function y2lat(y: number, zoom: number): number {
    const n = Math.PI - (2 * Math.PI * y) / Math.pow(2, zoom)
    return (180 / Math.PI) * Math.atan(0.5 * (Math.exp(n) - Math.exp(-n)))
}

/**
 * Calculates the longitude of a tile given its x position and zoom level
 * @param x - the x coordinate of a tile
 * @param zoom the map zoom level
 * @returns the corresponding longitude
 */
export function x2lon(x: number, zoom: number): number {
    return (x / Math.pow(2, zoom)) * 360 - 180
}

/**
 * Calculates the y part of a tile from a latitude and zoom level.
 * @param lat - a latitude
 * @param zoom - a map zoom level
 * @returns the corresponding y position
 */
export function lat2y(lat: number, zoom: number): number {
    const latRad = (lat * Math.PI) / 180
    return Math.floor(((1 - Math.log(Math.tan(latRad) + 1 / Math.cos(latRad)) / Math.PI) / 2) * (1 << zoom))
}

/**
 * Calculates the x part of a tile from a longitude and zoom level.
 * @param lon - a longitude
 * @param zoom - a map zoom level
 * @returns the corresponding x position
 */
export function lon2x(lon: number, zoom: number): number {
    return Math.floor(((lon + 180) / 360) * (1 << zoom))
}

type ExplorerLinesProps = {
    tileZoom: number
    lineColor: string
}

export function ExplorerLines({ tileZoom, lineColor }: ExplorerLinesProps) {
    const [lineCoordsArray, setLineCoordsArray] = useState<Array<Coords[]>>([])
    const [mapZoom, setMapZoom] = useState<number|null>(null)
    const [mapBounds, setMapBounds] = useState<LatLngBounds|null>(null)

    function setMapProps() {
        setMapZoom(map.getZoom())
        setMapBounds(map.getBounds())
    }

    const map = useMapEvents({
        moveend: () => setMapProps(),
        zoomend: () => setMapProps()
    })

    // The 'load' event seems to be fired too early, so listen to the whenReady callback for the initial load
    useEffect(() => {
        map.whenReady(() => setMapProps())
    }, [map])

    useEffect(() => {
        if (mapBounds) {
            const mapNorth = mapBounds.getNorth()
            const mapWest = mapBounds.getWest()
            const mapSouth = mapBounds.getSouth()
            const mapEast = mapBounds.getEast()
            // const tileNW = coords2tile([mapNorth, mapWest], zoom)
            // const tileNE = coords2tile([mapNorth, mapEast], zoom)
            // const tileSW = coords2tile([mapSouth, mapWest], zoom)

            const lineArray: Array<Coords[]> = []
            // Horizontal lines
            const yMin = lat2y(mapNorth, tileZoom) + 1
            const yMax = lat2y(mapSouth, tileZoom) + 1
            for (let y = yMin; y < yMax; y++) {
                const lat = y2lat(y, tileZoom)
                lineArray.push([[lat, mapWest], [lat, mapEast]])
            }
            // Vertical lines
            const xMin = lon2x(mapWest, tileZoom) + 1
            const xMax = lon2x(mapEast, tileZoom) + 1
            for (let x = xMin; x < xMax; x++) {
                const lon = x2lon(x, tileZoom)
                lineArray.push([[mapNorth, lon], [mapSouth, lon]])
            }
            setLineCoordsArray(lineArray)
        }
    }, [map, tileZoom, mapZoom, mapBounds])

    if (!mapZoom || mapZoom < tileZoom - 2) return

    return (
        <div>
            {lineCoordsArray.map((lineCoords, index) => (
                <Polyline key={index} positions={lineCoords} pathOptions={{ color: lineColor, weight: 2, opacity: 1 }} />
            ))}
        </div>
    )
}