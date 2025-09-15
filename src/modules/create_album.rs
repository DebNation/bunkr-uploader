use std::{collections::HashMap, io::stdin};

use crate::utils;

pub async fn create_album_fn(token: &str) -> String {
    let mut album_name = String::new();
    let mut album_desc = String::new();
    println!("Enter your album name: ");
    stdin().read_line(&mut album_name).unwrap();

    println!("Enter your album description: ");
    stdin().read_line(&mut album_desc).unwrap();

    let mut payload_hashmap = HashMap::new();
    payload_hashmap.insert("name", album_name.trim());
    payload_hashmap.insert("description", album_desc.trim());

    let resp = utils::api::create_album(&token, payload_hashmap)
        .await
        .unwrap();

    let message = resp.description.unwrap_or_default();
    let id = resp.id.unwrap_or_default();
    if !resp.success {
        panic!("{message}");
    }

    let formatted_id = format!("{}", id);
    return formatted_id;
}
