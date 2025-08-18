import { Pane, Rectangle } from 'react-leaflet'
import { Tile } from 'tiles-math'
import { TileArray, TileCache } from './TileCache.ts'

type TileOverlaysProps = {
    tileCache: TileCache
    tileColors: Array<string>
}

export function TileOverlays({ tileCache, tileColors }: TileOverlaysProps) {
    const overlays = Array.from(tileCache, ([zoom, tiles], index) =>
        <TileOverlay key={index} tiles={tiles} zoom={zoom} color={tileColors[index]} pane={index} />
    )
    return <div>{...overlays}</div>
}

type TileOverlayProps = {
    tiles: TileArray
    zoom: number
    color: string
    pane: number
}

function TileOverlay({ tiles, zoom, color, pane }: TileOverlayProps) {
    return (
        <Pane name={`pane-${pane}`} style={{ zIndex: 500 + pane }}>
            {tiles.map(tuple => Tile.of(tuple[0], tuple[1], zoom)).map((tile, index) =>
                <Rectangle key={index} bounds={tile.bounds()}
                           pathOptions={{color, weight: 0.5, opacity: 0.5}}/>
            )}
        </Pane>
    )
}
