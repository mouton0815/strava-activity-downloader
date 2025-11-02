import { Tile, TileSet } from 'tiles-math'
import { Pane, Rectangle } from 'react-leaflet'
import { useTileStore } from './useTileStore.ts'
import { useCandidateStore } from './useCandidateStore.ts'

type TilePanesProps = {
    zoomLevels: Array<number>
    tileColors: Array<string>
}

export function TilePanes({ zoomLevels, tileColors }: TilePanesProps) {
    const tileStore = useTileStore(state => state.tileStore)
    const candStore = useCandidateStore(state => state.candStore)
    const panes = zoomLevels.map((zoom, index) =>
        <TilePane key={index} tileSet={tileStore.get(zoom)} candSet={candStore.get(zoom)} color={tileColors[index]} pane={index} />
    )
    return <div>{...panes}</div>
}

type TilePaneProps = {
    tileSet: TileSet
    candSet: TileSet
    color: string
    pane: number
}

function TilePane({ tileSet, candSet, color, pane }: TilePaneProps) {
    const regularTiles = tileSet.map((tile: Tile, index) =>
        <Rectangle key={index} bounds={tile.bounds()}
                   pathOptions={{color, weight: 0.5, opacity: 0.5}}/>
    )
    const candidateTiles = candSet.map((tile: Tile, index) =>
        <Rectangle key={index} bounds={tile.bounds()}
                   pathOptions={{color, weight: 1, opacity: 1, fillOpacity: 0.3}}/>
    )
    return (
        <Pane name={`pane-${pane}`} style={{ zIndex: 500 + pane }}>
            {regularTiles}
            {candidateTiles}
        </Pane>
    )
}

