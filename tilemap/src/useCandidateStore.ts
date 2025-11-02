import { TileStore } from './TileStore.ts'
import { create } from 'zustand'

interface CandidateState {
    candStore: TileStore
    setCandStore: (candStore: TileStore) => void
}

export const useCandidateStore = create<CandidateState>((set) => ({
    candStore: new TileStore(),
    setCandStore: (candStore) => set({ candStore })
}))