import { LatLngTuple } from 'leaflet'
import { MapContainer, TileLayer } from 'react-leaflet'
import { LocationMarker } from './LocationMarker.tsx'
import { ExplorerLines } from './ExplorerLines.tsx'
import { TilePanes } from './TilePanes.tsx'
import { GPSTrackPolyline } from './GPSTrackPolyline.tsx'
import './App.css'

// This app and the tiles are delivered by the same Rust server.
// In dev mode, requests are passed through a proxy, see vite.config.js.
const TILES_URL = '/tiles'

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
            <TilePanes tilesUrl={TILES_URL} zoomLevels={ZOOM_LEVELS} tileColors={TILE_COLORS} />
            <GPSTrackPolyline />
            <LocationMarker crossHairSize={CROSSHAIR_SIZE} />
            <ExplorerLines zoomLevels={ZOOM_LEVELS} lineColors={TILE_COLORS} />
        </MapContainer>
    )
}
