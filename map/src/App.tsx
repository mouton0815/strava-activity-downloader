import { Suspense, use, useEffect, useState } from 'react'
import { divIcon, LatLng, LatLngBounds, LatLngTuple, marker } from 'leaflet'
import { MapContainer, Rectangle, TileLayer, useMapEvents } from 'react-leaflet'
import { coords2tile, Tile, TileNo } from 'tiles-math'
import './App.css'

const SERVER_URL = 'http://localhost:2525' // Base URL of the Rust server, use http://localhost:2525 in dev mode
const TILES_URL = `${SERVER_URL}/tiles`

const TILE_ZOOM = 14
const CROSSHAIR_SIZE = 50
const DEFAULT_CENTER: LatLngTuple = [51.33962, 12.37129] // Leipzig (will be relocated if user gives consent)

type TileTuple = [number, number] // Tile [x,y] as delivered by the REST endpoint

class TileBounds {
    x1: number
    y1: number
    x2: number
    y2: number
    constructor(upperLeft: TileNo, lowerRight: TileNo) {
        this.x1 = upperLeft.x
        this.y1 = upperLeft.y
        this.x2 = lowerRight.x
        this.y2 = lowerRight.y
    }
    static fromLatLngBounds(bounds: LatLngBounds): TileBounds {
        return new TileBounds(
            coords2tile([bounds.getNorth(), bounds.getWest()], TILE_ZOOM),
            coords2tile([bounds.getSouth(), bounds.getEast()], TILE_ZOOM))
    }
}

// TODO: Clip to map screen and reload if needed?
async function loadTiles(bounds: TileBounds | null): Promise<Array<TileTuple>> {
    // return Promise.resolve([[8755,5460],[8755,5461]])
    if (bounds) {
        const boundsParam = `bounds=${bounds.x1 + 1},${bounds.y1 + 1},${bounds.x2 - 1},${bounds.y2 - 1}`
        try {
            const response = await fetch(`${TILES_URL}/${TILE_ZOOM}?${boundsParam}`)
            return await response.json()
        } catch (e) {
            console.warn('Cannot fetch data from server:', e)
        }
    }
    return []
}

export function App() {
    return (
        <MapContainer
            zoomSnap={0.1}
            center={DEFAULT_CENTER}
            zoom={11}
            scrollWheelZoom={true}
            style={{ height: '100vh', minWidth: '100vw' }}>
            <TileLayer
                attribution='&copy; <a href="http://osm.org/copyright">OpenStreetMap</a> contributors'
                url="https://{s}.tile.openstreetmap.org/{z}/{x}/{y}.png"
            />
            <LoadContainer />
        </MapContainer>
    )
}

function LoadContainer() {
    const [location, setLocation] = useState<LatLng | null>(null)
    const [bounds, setBounds] = useState<TileBounds | null>(null)

    const map = useMapEvents({
        click: () => {
            // console.log('-----> locate ...')
            map.locate({setView: true, maxZoom: TILE_ZOOM})
        },
        locationfound: (event) => {
            // console.log('-----> location found:', event)
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
        },
        moveend: () => {
            // console.log('-----> moved')
            setBounds(TileBounds.fromLatLngBounds(map.getBounds()))
        },
        zoomend: () => {
            // console.log('-----> zoomed')
            setBounds(TileBounds.fromLatLngBounds(map.getBounds()))
        },
        viewreset: () => {
            // console.log('-----> reset')
            setBounds(TileBounds.fromLatLngBounds(map.getBounds()))
        }
    })

    useEffect(() => {
        if (location === null) {
            // console.log('-----> locate ...')
            map.locate({setView: true, maxZoom: TILE_ZOOM})
            setLocation(map.getCenter())
        }
    }, [location])

    // console.log("-----> PASS with ", bounds)
    return (
        <Suspense fallback={<div>Loading...</div>}>
            <TileContainer tilesPromise={loadTiles(bounds)} />
        </Suspense>
    )
}

type TileContainerProps = {
    tilesPromise: Promise<Array<TileTuple>>
}

function TileContainer({ tilesPromise }: TileContainerProps) {
    const tiles = use(tilesPromise)
    return (
        <div>
            {tiles.map(tuple => Tile.of(tuple[0], tuple[1], TILE_ZOOM)).map((tile, index) =>
                <Rectangle key={index} bounds={tile.bounds()}
                           pathOptions={{color: 'blue', weight: 0.5, opacity: 0.5}}/>
            )}
        </div>
   )
}
