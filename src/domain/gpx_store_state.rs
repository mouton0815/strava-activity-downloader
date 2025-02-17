#[allow(dead_code)] // Pending state is set implicitly by database
pub enum GpxStoreState {
    Pending = 0, // Track storage pending
    Stored  = 1, // Track stored
    Missing = 2  // Activity w/o track
}