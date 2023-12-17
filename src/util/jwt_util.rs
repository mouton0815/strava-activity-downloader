use std::error::Error;
use jsonwebtoken::{decode, DecodingKey, Validation};
use oauth2::TokenResponse;
use oauth2::basic::BasicTokenResponse;
use serde::Deserialize;
use log::info;

#[derive(Deserialize)]
struct Claims {
    exp: u64
}

pub fn get_expiry_time(token: &BasicTokenResponse) -> Result<u64, Box<dyn Error>> {
    let token = token.access_token().secret();
    info!("--t--> {}", token);
    let mut validation = Validation::default();
    // This is NOT insecure because the JWT was just received from the Auth server:
    validation.insecure_disable_signature_validation();
    validation.validate_aud = false;
    let token = decode::<Claims>(token, &DecodingKey::from_secret(&[]), &validation)?;
    info!("--t--> {:?}", token.header);
    info!("--t--> {}", token.claims.exp);
    Ok(token.claims.exp)
}
