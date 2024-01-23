use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub enum DownloadState {
    Inactive,
    Activities,
    Tracks
}
