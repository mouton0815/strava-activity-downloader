import { LatLngTuple } from 'leaflet'
import { MapContainer, TileLayer } from 'react-leaflet'
import { LocationMarker } from './LocationMarker.tsx'
import { ExplorerLines } from './ExplorerLines.tsx'
import { LocationWatcher } from './LocationWatcher.tsx'
import { TileFetcher } from './TileFetcher.tsx'
import { TileDetector } from './TileDetector.tsx'
import { TilePanes } from './TilePanes.tsx'
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
            <TileFetcher tilesUrl={TILES_URL} zoomLevels={ZOOM_LEVELS} />
            <TileDetector zoomLevels={ZOOM_LEVELS} />
            <TilePanes zoomLevels={ZOOM_LEVELS} tileColors={TILE_COLORS} />
            <ExplorerLines zoomLevels={ZOOM_LEVELS} lineColors={TILE_COLORS} />
        </MapContainer>
    )
}
