use log::info;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct ErrorResult {
    error: String
}

pub async fn login() {
    info!("Logging in!");
}