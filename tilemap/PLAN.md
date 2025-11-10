# GPS Position Tracking and Visualization Implementation Plan

## Overview
Add functionality to track all GPS positions visited during app runtime (with 10-meter minimum distance filtering) and visualize them as a Leaflet polyline on the map.

## Current State Analysis
- The app uses `useLocation()` hook to track current GPS position (from `useLocation.ts`)
- GPS position updates trigger tile detection in `useDetectedTiles.ts`
- `LocationMarker.tsx` displays current position and pans the map
- `ExplorerLines.tsx` already demonstrates drawing polylines on the map using Leaflet's `Polyline` component
- Leaflet and react-leaflet are already available dependencies

## Implementation Strategy

### 1. Create Position Tracking Hook (`usePositionTracker.ts`)
**Purpose**: Track all GPS positions with 10-meter minimum distance filtering
**Location**: `src/usePositionTracker.ts`

**Responsibilities**:
- Accept the current location from `useLocation()` hook
- Maintain an array of recorded positions (LatLngTuple format)
- Calculate distance between current position and last recorded position using Haversine formula
- Only add position to array if distance >= 10 meters
- Return the array of tracked positions
- Use `useEffect` to monitor location changes
- Use `useState` to manage positions array

**Key Logic**:
- Haversine distance calculation between two [lat, lng] coordinates
- Store positions in `[lat, lng]` format compatible with Leaflet's Polyline
- Efficiently update array only when threshold is exceeded

**Dependencies**:
- React hooks: `useState`, `useEffect`
- Leaflet types: `LatLngTuple`

### 2. Create GPS Track Visualization Component (`GPSTrackPolyline.tsx`)
**Purpose**: Track GPS positions and render as a Leaflet polyline on the map
**Location**: `src/GPSTrackPolyline.tsx`

**Responsibilities**:
- Call `usePositionTracker()` hook internally to get tracked positions
- Render positions using Leaflet's `Polyline` component
- Apply distinctive styling (color, weight, opacity) to differentiate from tile grids
- Handle empty position arrays gracefully (return null)
- Update polyline reactively when new positions are added

**Design Decisions**:
- Use a distinct color (e.g., red or purple) to differentiate from tile grids (blue/green)
- Use appropriate weight/opacity to be visible but not obtrusive
- Update in real-time as positions are added
- Encapsulate both tracking logic and visualization in single component

**Dependencies**:
- React: `ReactNode` or functional component
- react-leaflet: `Polyline`
- Leaflet types: `LatLngTuple`
- Local hook: `usePositionTracker`

**Component API**:
- No props required (self-contained)

### 3. Integrate into App Component (`App.tsx`)
**Modifications**:
- Import and render `GPSTrackPolyline` component only
- Place polyline rendering after TilePanes but before or after LocationMarker for proper z-ordering

**Integration Points**:
- `<GPSTrackPolyline />` component (no props needed)
- Component internally calls `usePositionTracker()` hook
- Single component encapsulates both tracking logic and visualization

**Design Benefit**:
- Cleaner App.tsx with self-contained GPS tracking component
- Decouples tracking from App state management
- Hook remains local to where it's used

### 4. Distance Calculation Helper (Within `usePositionTracker.ts`)
**Haversine Formula Implementation**:
- Function signature: `calculateDistance(lat1: number, lng1: number, lat2: number, lng2: number): number`
- Returns distance in meters
- Formula: Standard Haversine distance calculation
- Earth radius: 6,371,000 meters

### 5. Data Structure for Positions
**Format**: `LatLngTuple[]` (already used by Leaflet)
- Each position: `[latitude: number, longitude: number]`
- Lightweight and directly compatible with Leaflet's Polyline

## File Changes Summary

### New Files to Create:
1. **`src/usePositionTracker.ts`**
   - Position tracking logic with 10-meter threshold
   - Haversine distance calculation
   - Returns filtered positions array

2. **`src/GPSTrackPolyline.tsx`**
   - Polyline rendering component
   - Visualizes tracked positions on map

### Files to Modify:
1. **`src/App.tsx`**
   - Import and render `GPSTrackPolyline` component (no hook import needed)

### No Changes Required:
- `useLocation.ts` - already provides location updates
- `LocationMarker.tsx` - continues to mark current position
- `ExplorerLines.tsx` - continues to show tile grids
- `TileStore.ts`, `TileBounds.ts` - support tile tracking
- `useDetectedTiles.ts` - continues tile detection

## Polyline Styling Recommendations
- **Color**: Red (or purple/orange) to distinguish from tile grids
- **Weight**: 2-3 pixels (visible but not overwhelming)
- **Opacity**: 0.7-0.8 (allows map to show through)
- **Dash array**: Consider dashed line to further differentiate from grids

## Performance Considerations
1. **Memory**: Positions array grows over time; consider adding optional max-size limit or persistence strategy for long sessions
2. **Distance calculations**: Haversine computation is O(1) per location update, negligible overhead
3. **Polyline rendering**: Leaflet efficiently handles polylines with hundreds of points
4. **Data consumption**: ✓ Minimized by filtering with 10-meter threshold

## Future Enhancement Possibilities
- Export tracked path as GPX/GeoJSON format
- Persist positions to localStorage for session recovery
- Add start/stop button to pause tracking
- Display total distance traveled
- Add altitude tracking if available from Geolocation API
- Animate along the tracked path

## Testing Checklist
- [ ] Position tracking starts on app load
- [ ] No position recorded until 10m distance threshold exceeded
- [ ] Polyline appears on map with correct styling
- [ ] Multiple positions tracked create continuous polyline
- [ ] Positions persist and polyline updates correctly during continued app use
- [ ] No data sent to server (frontend-only tracking)
- [ ] Performance remains acceptable during extended tracking
