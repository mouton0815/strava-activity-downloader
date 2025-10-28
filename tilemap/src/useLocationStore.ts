import { create } from 'zustand'
import { LatLng } from 'leaflet'

interface LocationState {
    position: LatLng | null
    setPosition: (pos: LatLng | null) => void
}

export const useLocationStore = create<LocationState>((set) => ({
    position: null,
    setPosition: (position) => set({ position })
}))