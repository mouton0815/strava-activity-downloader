#[derive(Clone, Debug)]
#[allow(dead_code)] // Pending state is set implicitly by database
pub enum TrackStoreState {
    Pending = 0, // Track storage pending
    Stored  = 1, // Track stored
    Missing = 2  // Activity w/o track
}