import { Tile, TileSet } from 'tiles-math'
import { Pane, Rectangle } from 'react-leaflet'
import { useFetchedTiles } from './useFetchedTiles.ts'
import { useDetectedTiles } from './useDetectedTiles.ts'

type TilePanesProps = {
    zoomLevels: Array<number>
    tileColors: Array<string>
}

export function TilePanes({ zoomLevels, tileColors }: TilePanesProps) {
    const fetchedTiles = useFetchedTiles(state => state.fetchedTiles)
    const detectedTiles = useDetectedTiles(state => state.detectedTiles)
    const panes = zoomLevels.map((zoom, index) =>
        <TilePane key={index} fetchedSet={fetchedTiles.get(zoom)} detectedSet={detectedTiles.get(zoom)} tileColor={tileColors[index]} paneIndex={index} />
    )
    return <div>{...panes}</div>
}

type TilePaneProps = {
    fetchedSet: TileSet
    detectedSet: TileSet
    tileColor: string
    paneIndex: number
}

function TilePane({ fetchedSet, detectedSet, tileColor, paneIndex }: TilePaneProps) {
    const fetchedTiles = fetchedSet.map((tile: Tile, index) =>
        <Rectangle key={index} bounds={tile.bounds()}
                   pathOptions={{color: tileColor, weight: 0.5, opacity: 0.5}}/>
    )
    const detectedTiles = detectedSet.map((tile: Tile, index) =>
        <Rectangle key={index} bounds={tile.bounds()}
                   pathOptions={{color: tileColor, weight: 1, opacity: 1, fillOpacity: 0.3}}/>
    )
    return (
        <Pane name={`pane-${paneIndex}`} style={{ zIndex: 500 + paneIndex }}>
            {fetchedTiles}
            {detectedTiles}
        </Pane>
    )
}

