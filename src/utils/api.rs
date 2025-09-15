use std::{collections::HashMap, error::Error};

use colored::Colorize;
use serde::Deserialize;

use reqwest::Client;
use serde_json::Value;

pub async fn get_data(token: &str) -> Result<Value, Box<dyn std::error::Error>> {
    let client = Client::new();
    let response = client
        .get("https://dash.bunkr.cr/api/node")
        .header("token", token)
        .send()
        .await?
        .text()
        .await?;

    let json: Value = serde_json::from_str(&response)?;
    Ok(json)
}

#[derive(Debug, Deserialize)]
pub struct VerifyTokenResp {
    pub success: bool,
}

pub async fn verify_token(token: &str) -> Result<VerifyTokenResp, Box<dyn std::error::Error>> {
    let client = Client::new();
    let mut payload_hashmap = HashMap::new();
    payload_hashmap.insert("token", token);
    let response = client
        .post("https://dash.bunkr.cr/api/tokens/verify")
        .json(&payload_hashmap)
        .send()
        .await?
        .text()
        .await?;

    let json: VerifyTokenResp = serde_json::from_str(&response)?;
    Ok(json)
}

#[derive(Debug, Deserialize)]
pub struct AlbumResponse {
    pub albums: Vec<Album>,
}

#[derive(Debug, Deserialize)]
pub struct Album {
    pub id: u32,
    pub name: String,
}

pub async fn get_albums(token: &str) -> Result<AlbumResponse, Box<dyn Error>> {
    println!("{}", "Fetching Albums...".green());
    let client = Client::new();
    let response = client
        .get("https://dash.bunkr.cr/api/albums")
        .header("token", token)
        .send()
        .await?
        .text()
        .await?;
    let json: AlbumResponse = serde_json::from_str(&response)?;
    Ok(json)
}

#[derive(Debug, Deserialize)]
pub struct AlbumCreateResponse {
    pub success: bool,
    pub id: Option<u32>,
    pub description: Option<String>,
}

// #[derive(Debug, Deserialize)]
// pub struct AlbumCreatePayload {
//     pub name: String,
//     pub description: String,
// }
pub async fn create_album(
    token: &str,
    payload_hashmap: HashMap<&str, &str>,
) -> Result<AlbumCreateResponse, Box<dyn Error>> {
    println!("{}", "Creating Album...".green());
    let client = Client::new();
    let response = client
        .post("https://dash.bunkr.cr/api/albums")
        .json(&payload_hashmap)
        .header("token", token)
        .send()
        .await?
        .text()
        .await?;
    let json: AlbumCreateResponse = serde_json::from_str(&response)?;
    Ok(json)
}
