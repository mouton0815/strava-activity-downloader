import { useEffect, useState } from 'react'
import { Polyline, useMapEvents } from 'react-leaflet'
import { LatLngBounds, Map } from 'leaflet'
import { Coords, lat2y, lon2x, x2lon, y2lat } from 'tiles-math'

type ExplorerLineProps = {
    zoomLevels: Array<number>
    lineColors: Array<string>
}

/**
 * Draws grids on the map for the given tile zoom levels.
 * The grids get disabled when the map is zoomed out.
 */
export function ExplorerLines({ zoomLevels, lineColors }: ExplorerLineProps) {
    const grids = zoomLevels.map((zoom, index) =>
        <ExplorerGrid key={index} tileZoom={zoom} lineColor={lineColors[index]} />
    )
    return <div>{...grids}</div>
}

type ExplorerGridProps = {
    tileZoom: number
    lineColor: string
}

/**
 * Draws a grid for the given tile zoom level.
 */
function ExplorerGrid({ tileZoom, lineColor }: ExplorerGridProps) {
    const [lineCoordsArray, setLineCoordsArray] = useState<Array<Coords[]>>([])
    const [mapZoom, setMapZoom] = useState<number|null>(null)
    const [mapBounds, setMapBounds] = useState<LatLngBounds|null>(null)

    // Convenience function
    function setMapProps(map: Map) {
        setMapZoom(map.getZoom())
        setMapBounds(map.getBounds())
    }

    const map = useMapEvents({
        moveend: () => setMapProps(map),
        zoomend: () => setMapProps(map)
    })

    // The 'load' event seems to be fired too early, so listen to the whenReady callback for the initial load
    useEffect(() => {
        map.whenReady(() => setMapProps(map))
    }, [map])

    useEffect(() => {
        if (mapBounds) {
            const mapNorth = mapBounds.getNorth()
            const mapWest = mapBounds.getWest()
            const mapSouth = mapBounds.getSouth()
            const mapEast = mapBounds.getEast()

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
                <Polyline key={index} positions={lineCoords} pathOptions={{ color: lineColor, weight: 0.5, opacity: 1 }} />
            ))}
        </div>
    )
}