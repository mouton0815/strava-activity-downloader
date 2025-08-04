import { Suspense, use } from 'react'
import { divIcon, marker } from 'leaflet'
import { MapContainer, Rectangle, TileLayer, useMap } from 'react-leaflet'
import { Coords, Tile } from 'tiles-math'
import './App.css'

const SERVER_URL = 'http://localhost:2525' // Base URL of the Rust server, use http://localhost:2525 in dev mode
const TILES_URL = `${SERVER_URL}/tiles`

const TILE_ZOOM = 14
const CROSSHAIR_SIZE = 50

// TODO: Clip to map screen and reload if needed?
async function loadTiles(): Promise<Array<Coords>> {
    // return Promise.resolve([[8755,5460],[8755,5461]])
    try {
        const response = await fetch(`${TILES_URL}/${TILE_ZOOM}`)
        return await response.json()
    } catch (e) {
        console.warn('Cannot fetch data from server:', e)
        return []
    }
}

export function App() {
    return (
        <MapContainer
            zoomSnap={0.1}
            center={[51.33962, 12.37129]}
            zoom={11}
            scrollWheelZoom={true}
            style={{ height: '100vh', minWidth: '100vw' }}>
            <TileLayer
                attribution='&copy; <a href="http://osm.org/copyright">OpenStreetMap</a> contributors'
                url="https://{s}.tile.openstreetmap.org/{z}/{x}/{y}.png"
            />
            <Suspense fallback={<div>Loading...</div>}>
                <TileContainer tilesPromise={loadTiles()} />
            </Suspense>
        </MapContainer>
    )
}

type TileContainerProps = {
    tilesPromise: Promise<Array<Coords>>
}

function TileContainer({ tilesPromise }: TileContainerProps) {
    const tiles = use(tilesPromise)
    //console.log('----->', data)
    const map = useMap()
    map.on('locationfound', (e) => {
        const icon = divIcon({
            className: 'crosshair-marker',
            iconSize: [CROSSHAIR_SIZE, CROSSHAIR_SIZE],
            iconAnchor: [CROSSHAIR_SIZE / 2, CROSSHAIR_SIZE / 2] // Centered
        })
        marker(e.latlng, { icon }).addTo(map);
        //circle(e.latlng, { radius: 50 }).addTo(map)
    })
    map.on('locationerror', (e) => {
        console.warn(e.message)
        alert(e.message)
    })
    map.locate({setView: true, maxZoom: TILE_ZOOM})

    return (
        <div>
            {tiles.map(coords => Tile.of(coords[0], coords[1], TILE_ZOOM)).map((tile, index) =>
                <Rectangle key={index} bounds={tile.bounds()}
                           pathOptions={{color: 'blue', weight: 0.5, opacity: 0.5}}/>
            )}
        </div>
   )
}
