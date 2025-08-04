use base64::{Engine as _, engine::general_purpose};
use std::io::Write;
use std::{
    fs::{self},
    io::{self},
};
async fn get_actual_token(token: String, token_file_path: String) -> String {
    let mut verified: bool = false;
    let mut prev_valid_token: bool = true;
    let mut new_token: String = Default::default();

    while !verified {
        if token.is_empty() || !prev_valid_token {
            io::stdout().flush().unwrap();
            println!("Enter token: ");
            let mut input_token: String = String::new();
            io::stdin().read_line(&mut input_token).unwrap();
            new_token = input_token.trim().to_string();
            if new_token.is_empty() {
                eprintln!("Token can't be empty");
                continue;
            }
        } else {
            new_token = token.clone();
        }

        let is_token_verified = match super::api::verify_token(&new_token).await {
            Ok(data) => data.success,
            Err(e) => {
                eprintln!("{:?}", e);
                continue;
            }
        };

        if !is_token_verified {
            eprintln!("Invalid Token");
            prev_valid_token = false;
            continue;
        }
        let b64 = general_purpose::STANDARD.encode(&new_token);
        fs::write(&token_file_path, b64).unwrap();
        verified = true;
    }
    new_token
}

pub async fn handle_token(token_file_path: String) -> String {
    let token = match fs::read_to_string(&token_file_path) {
        Ok(content) => {
            let b64_decoded = general_purpose::STANDARD.decode(&content).unwrap();
            String::from_utf8(b64_decoded).unwrap()
        }

        Err(err) => {
            return Err(err).expect("Failed to parse the token");
        }
    };
    let verified_token = get_actual_token(token, token_file_path).await;
    // println!("Token Verified");
    verified_token
}
