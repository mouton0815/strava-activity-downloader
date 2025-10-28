import { LatLngTuple } from 'leaflet'
import { MapContainer, TileLayer } from 'react-leaflet'
import { LocationMarker } from './LocationMarker.tsx'
import { TileContainer } from './TileContainer.tsx'
import { ExplorerLines } from './ExplorerLines.tsx'
import { LocationWatcher } from './LocationWatcher.tsx'
import './App.css'

// Base URL of the Rust server.
// Use an empty string if this frontend is delivered by the Rust server.
// Use 'http://localhost:2525' if this frontend runs in dev mode (`npm run dev`).
const SERVER_URL = '' // 'http://localhost:2525'
const TILES_URL = `${SERVER_URL}/tiles`

const ZOOM_LEVELS = [14, 17]
const TILE_COLORS = ['blue', 'green'] // Tile colors for the zoom levels

const CROSSHAIR_SIZE = 50

const DEFAULT_CENTER: LatLngTuple = [51.33962, 12.37129] // [0.0, 0.0]

export function App() {
    return (
        <MapContainer
            zoomSnap={0.1}
            center={DEFAULT_CENTER}
            zoom={14}
            scrollWheelZoom={true}
            style={{ height: '100vh', minWidth: '100vw' }}>
            <TileLayer
                attribution='&copy; <a href="http://osm.org/copyright">OpenStreetMap</a> contributors'
                url="https://{s}.tile.openstreetmap.org/{z}/{x}/{y}.png"
            />
            <LocationWatcher />
            <LocationMarker crossHairSize={CROSSHAIR_SIZE} />
            <TileContainer tilesUrl={TILES_URL} zoomLevels={ZOOM_LEVELS} tileColors={TILE_COLORS}/>
            <ExplorerLines tileZoom={14} lineColor={'blue'} />
            <ExplorerLines tileZoom={17} lineColor={'green'} />
        </MapContainer>
    )
}
