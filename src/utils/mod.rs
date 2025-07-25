use std::collections::HashMap;

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

pub async fn verify_token(token: &str) -> Result<Value, Box<dyn std::error::Error>> {
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

    let json: Value = serde_json::from_str(&response)?;
    Ok(json)
}
