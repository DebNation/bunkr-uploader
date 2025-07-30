use base64::{Engine as _, engine::general_purpose};
use std::io::Write;
use std::{
    fs::{self},
    io::{self},
};
async fn retry_token(mut token: String, token_file_path: String) -> String {
    while token.is_empty() {
        io::stdout().flush().unwrap();
        println!("Enter token: ");
        let mut input_token: String = String::new();
        io::stdin().read_line(&mut input_token).unwrap();
        let trimmed_token = input_token.trim();
        if trimmed_token.is_empty() {
            eprintln!("Token can't be empty");
            continue;
        }

        let is_token_verified = match super::api::verify_token(&trimmed_token).await {
            Ok(data) => data.success,
            Err(e) => {
                eprintln!("{:?}", e);
                continue;
            }
        };

        if !is_token_verified {
            eprintln!("Invalid Token Entered");
            continue;
        }
        let b64 = general_purpose::STANDARD.encode(&trimmed_token);
        fs::write(&token_file_path, b64).unwrap();
        token = trimmed_token.to_string();
    }
    token.to_string()
}

pub async fn handle_token(token_file_path: String) -> String {
    let token = match fs::read_to_string(&token_file_path) {
        Ok(content) => {
            let b64_decoded = general_purpose::STANDARD.decode(&content).unwrap();
            String::from_utf8(b64_decoded).unwrap()
        }

        Err(e) => {
            eprintln!("failed to parse token, {}", e);
            Default::default()
        }
    };
    retry_token(token.to_string(), token_file_path).await;
    token
}
