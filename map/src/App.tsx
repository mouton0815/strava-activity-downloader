import { Suspense, use, useEffect, useState } from 'react'
import { divIcon, LatLng, LatLngBounds, LatLngTuple, marker } from 'leaflet'
import { MapContainer, Pane, Rectangle, TileLayer, useMapEvents } from 'react-leaflet'
import { coords2tile, Tile, TileNo } from 'tiles-math'
import './App.css'

const SERVER_URL = 'http://localhost:2525' // Base URL of the Rust server, use http://localhost:2525 in dev mode
const TILES_URL = `${SERVER_URL}/tiles`

const TILE_ZOOM_LEVELS = [14, 17]
const TILE_ZOOM_COLORS = ['blue', 'yellow']
const CROSSHAIR_SIZE = 50
const DEFAULT_CENTER: LatLngTuple = [51.33962, 12.37129] // Leipzig (will be relocated if user gives consent)

type TileTuple = [number, number] // Tile [x,y] as delivered by the REST endpoint
type TileArray = Array<TileTuple>
type TileArrayMap = Map<number, TileArray> // zoom -> [tile, tile, ...]

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
    static fromLatLngBounds(bounds: LatLngBounds, zoom: number): TileBounds {
        return new TileBounds(
            coords2tile([bounds.getNorth(), bounds.getWest()], zoom),
            coords2tile([bounds.getSouth(), bounds.getEast()], zoom)
        )
    }
    contains(that: TileBounds): boolean {
        return this.x1 <= that.x1 && this.y1 <= that.y1 && this.x2 >= that.x2 && this.y2 >= that.y2
    }
}

class TileBoundsMap {
    map: Map<number, TileBounds>
    constructor(map: Map<number, TileBounds> | null = null) {
        this.map = map || new Map<number, TileBounds>()
    }
    static fromLatLngBounds(bounds: LatLngBounds): TileBoundsMap {
        const map = new Map<number, TileBounds>()
        for (const zoom of TILE_ZOOM_LEVELS) {
            map.set(zoom, TileBounds.fromLatLngBounds(bounds, zoom))
        }
        return new TileBoundsMap(map)
    }
    get(zoom: number): TileBounds {
        const bounds = this.map.get(zoom)
        if (!bounds) {
            throw new Error(`Zoom level ${zoom} not available in map (this is a bug`)
        }
        return bounds
    }
    set(zoom: number, bounds: TileBounds) {
        this.map.set(zoom, bounds)
    }
    contains(bounds: TileBounds, zoom: number): boolean {
        const thisBounds = this.map.get(zoom)
        return !!thisBounds && thisBounds.contains(bounds)
    }
}


// TODO: Could they be managed as React state?
const tileCache: TileArrayMap = new Map<number, TileArray>()
const prevBounds: TileBoundsMap = new TileBoundsMap()

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

async function loadTileCache(boundsMap: TileBoundsMap | null): Promise<TileArrayMap> {
    // return Promise.resolve([[8755,5460],[8755,5461]])
    if (boundsMap) { // Null until the initial Leaflet event
        for (const zoom of TILE_ZOOM_LEVELS) {
            const bounds = boundsMap.get(zoom)
            if (!prevBounds.contains(bounds, zoom)) {
                const tiles = await loadTiles(bounds, zoom)
                tileCache.set(zoom, tiles)
                prevBounds.set(zoom, bounds)
            }
        }
    }
    return tileCache
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
    const [bounds, setBounds] = useState<TileBoundsMap | null>(null)

    const map = useMapEvents({
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
            setBounds(TileBoundsMap.fromLatLngBounds(map.getBounds()))
        },
        zoomend: () => {
            // console.log('-----> zoomed')
            setBounds(TileBoundsMap.fromLatLngBounds(map.getBounds()))
        },
        viewreset: () => {
            // console.log('-----> reset')
            setBounds(TileBoundsMap.fromLatLngBounds(map.getBounds()))
        }
    })

    useEffect(() => {
        if (location === null) {
            // console.log('-----> locate ...')
            map.locate({setView: true, maxZoom: 14})
            setLocation(map.getCenter())
        }
    }, [location])

    // console.log("-----> PASS with ", bounds)
    return (
        <Suspense fallback={<div>Loading...</div>}>
            <TileContainer tilesPromise={loadTileCache(bounds)} />
        </Suspense>
    )
}

type TileContainerProps = {
    tilesPromise: Promise<TileArrayMap>
}

function TileContainer({ tilesPromise }: TileContainerProps) {
    const tiles = use(tilesPromise)
    const overlays = Array.from(tiles, ([zoom, tiles], index) =>
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
