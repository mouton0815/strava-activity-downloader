import { Suspense, useEffect, useState } from 'react'
import { divIcon, LatLng, LatLngTuple, marker } from 'leaflet'
import { MapContainer, Pane, Rectangle, TileLayer, useMapEvents } from 'react-leaflet'
import { TileBounds, TileBoundsMap } from './TileBounds.ts'
import { Tile } from 'tiles-math'
import './App.css'

const SERVER_URL = 'http://localhost:2525' // Base URL of the Rust server, use http://localhost:2525 in dev mode
const TILES_URL = `${SERVER_URL}/tiles`

const TILE_ZOOM_LEVELS = [14, 17]
const TILE_ZOOM_COLORS = ['blue', 'green']
const CROSSHAIR_SIZE = 50
const DEFAULT_CENTER: LatLngTuple = [51.33962, 12.37129] // Leipzig (will be relocated if user gives consent)

type TileTuple = [number, number] // Tile [x,y] as delivered by the REST endpoint
type TileArray = Array<TileTuple>
type TileArrayMap = Map<number, TileArray> // zoom -> [tile, tile, ...]


async function loadTiles(bounds: TileBounds, zoom: number): Promise<TileArray> {
    try {
        const boundsParam = `bounds=${bounds.x1 + 2},${bounds.y1 + 2},${bounds.x2 - 2},${bounds.y2 - 2}`
        const response = await fetch(`${TILES_URL}/${zoom}?${boundsParam}`)
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
            center={DEFAULT_CENTER}
            zoom={11}
            scrollWheelZoom={true}
            style={{ height: '100vh', minWidth: '100vw' }}>
            <TileLayer
                attribution='&copy; <a href="http://osm.org/copyright">OpenStreetMap</a> contributors'
                url="https://{s}.tile.openstreetmap.org/{z}/{x}/{y}.png"
            />
            <TileLoadContainer />
        </MapContainer>
    )
}

function TileLoadContainer() {
    const [location, setLocation] = useState<LatLng | null>(null)
    const [newBounds, setNewBounds] = useState<TileBoundsMap | null>(null)
    const [maxBounds, setMaxBounds] = useState<TileBoundsMap>(new TileBoundsMap())
    const [tileCache, setTileCache] = useState<TileArrayMap>(new Map<number, TileArray>())

    // React on map events (to determine location and to determine visible map bounds for tile loading)
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
        },
        moveend: () => {
            setNewBounds(TileBoundsMap.fromLatLngBounds(map.getBounds(), TILE_ZOOM_LEVELS))
        },
        zoomend: () => {
            setNewBounds(TileBoundsMap.fromLatLngBounds(map.getBounds(), TILE_ZOOM_LEVELS))
        },
        viewreset: () => {
            setNewBounds(TileBoundsMap.fromLatLngBounds(map.getBounds(), TILE_ZOOM_LEVELS))
        }
    })

    // Determine GPS location of map
    useEffect(() => {
        if (location === null) {
            map.locate({setView: true, maxZoom: 14})
            setLocation(map.getCenter())
        }
    }, [location])

    // Load (and cache) tiles according to the visible map bounds
    useEffect(() => {
        let isCancelled = false

        async function fetchData() {
            if (newBounds) { // Null until the initial Leaflet event
                for (const zoom of TILE_ZOOM_LEVELS) {
                    const bounds = newBounds.get(zoom)
                    if (!maxBounds.contains(bounds, zoom)) {
                        const tiles = await loadTiles(bounds, zoom)
                        if (!isCancelled) {
                            tileCache.set(zoom, tiles)
                            maxBounds.set(zoom, bounds)
                        }
                    }
                }
                if (!isCancelled) {
                    setTileCache(new Map<number, TileArray>(tileCache)) // Shallow copy
                    setMaxBounds(maxBounds.shallowCopy())
                }
            }
        }

        fetchData()

        return () => {
            isCancelled = true;
        }
    }, [newBounds])

    // console.log("-----> PASS with ", bounds)
    return (
        <Suspense fallback={<div>Loading...</div>}>
            <TileContainer tileCache={tileCache} />
        </Suspense>
    )
}

type TileContainerProps = {
    tileCache: TileArrayMap
}

function TileContainer({ tileCache }: TileContainerProps) {
    const overlays = Array.from(tileCache, ([zoom, tiles], index) =>
        <TileOverlay key={index} tiles={tiles} zoom={zoom} pane={index} />
    )
    return <div>{...overlays}</div>
}

type TileOverlayProps = {
    tiles: TileArray
    zoom: number
    pane: number
}

function TileOverlay({ tiles, zoom, pane }: TileOverlayProps) {
    return (
        <Pane name={`pane-${pane}`} style={{ zIndex: 500 + pane }}>
            {tiles.map(tuple => Tile.of(tuple[0], tuple[1], zoom)).map((tile, index) =>
                <Rectangle key={index} bounds={tile.bounds()}
                           pathOptions={{color: TILE_ZOOM_COLORS[pane], weight: 0.5, opacity: 0.5}}/>
            )}
        </Pane>
    )
}
