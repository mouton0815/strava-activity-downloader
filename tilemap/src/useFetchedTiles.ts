import { TileStore } from './TileStore.ts'
import { create } from 'zustand'

interface FetchedTilesState {
    fetchedTiles: TileStore
    setFetchedTiles: (fetchedTiles: TileStore) => void
}

export const useFetchedTiles = create<FetchedTilesState>((set) => ({
    fetchedTiles: new TileStore(),
    setFetchedTiles: (fetchedTiles) => set({ fetchedTiles })
}))