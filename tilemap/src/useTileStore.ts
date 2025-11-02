import { TileStore } from './TileStore.ts'
import { create } from 'zustand'

interface TileStoreState {
    tileStore: TileStore
    setTileStore: (tiles: TileStore) => void
}

export const useTileStore = create<TileStoreState>((set) => ({
    tileStore: new TileStore(),
    setTileStore: (tiles) => set({ tileStore: tiles })
}))