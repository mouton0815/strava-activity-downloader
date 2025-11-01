import { Pane, Rectangle } from 'react-leaflet'
import { coords2tile, Tile, TileSet } from 'tiles-math'
import { TileCache } from './TileCache.ts'
import { useLocationStore } from './useLocationStore.ts'
import { useEffect, useState } from 'react'

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
    const position  = useLocationStore(state => state.position)
    const [candidates, setCandidates] = useState<TileSet>(new TileSet(tiles.getZoom()))

    useEffect(() => {
        if (position) {
            let changed = false
            for (const tileNo of candidates) {
                if (tiles.has(tileNo)) {
                    candidates.removeTile(tileNo)
                    changed = true
                }
            }
            const tileNo = coords2tile([position.lat, position.lng], tiles.getZoom())
            if (!candidates.has(tileNo) && !tiles.has(tileNo)) {
                candidates.addTile(tileNo)
                changed = true
            }
            if (changed) {
                setCandidates(candidates.clone()) // Shallow clone
            }
        }
    }, [position])

    const regularTiles = tiles.map((tile: Tile, index) =>
        <Rectangle key={index} bounds={tile.bounds()}
                   pathOptions={{color, weight: 0.5, opacity: 0.5}}/>
    )
    const candidateTiles = candidates.map((tile: Tile, index) =>
        <Rectangle key={index} bounds={tile.bounds()}
                   pathOptions={{color, weight: 1, opacity: 1, fillOpacity: 0.4}}/>
    )

    return (
        <Pane name={`pane-${pane}`} style={{ zIndex: 500 + pane }}>
            {regularTiles}
            {candidateTiles}
        </Pane>
    )
}
