import { Pane, Rectangle } from 'react-leaflet'
import { Tile, TileSet } from 'tiles-math'
import { TileCache } from './TileCache.ts'

type TileOverlaysProps = {
    tileCache: TileCache
    tileColors: Array<string>
}

export function TileOverlays({ tileCache, tileColors }: TileOverlaysProps) {
    const overlays = Array.from(tileCache, (tiles, index) =>
        <TileOverlay key={index} tiles={tiles} color={tileColors[index]} pane={index} />
    )
    return <div>{...overlays}</div>
}

type TileOverlayProps = {
    tiles: TileSet
    color: string
    pane: number
}

function TileOverlay({ tiles, color, pane }: TileOverlayProps) {
    return (
        <Pane name={`pane-${pane}`} style={{ zIndex: 500 + pane }}>
            {tiles.map((tile: Tile, index) =>
                <Rectangle key={index} bounds={tile.bounds()}
                           pathOptions={{color, weight: 0.5, opacity: 0.5}}/>
            )}
        </Pane>
    )
}
