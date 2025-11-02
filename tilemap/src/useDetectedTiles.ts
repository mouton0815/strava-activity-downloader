import { TileStore } from './TileStore.ts'
import { create } from 'zustand'

interface DetectedTilesState {
    detectedTiles: TileStore
    setDetectedTiles: (detectedTiles: TileStore) => void
}

export const useDetectedTiles = create<DetectedTilesState>((set) => ({
    detectedTiles: new TileStore(),
    setDetectedTiles: (detectedTiles) => set({ detectedTiles })
}))