use std::io::Write;
use std::{
    fs::{self},
    io::{self},
};

pub async fn verify_token(token_file_path: String) -> String {
    let mut token: String = String::new();
    match fs::read_to_string(&token_file_path) {
        Ok(content) => {
            println!("Verifying token");
            let is_token_verified: bool = match super::api::verify_token(&content).await {
                Ok(data) => {
                    let is_verified: bool = data["success"].to_string().parse().unwrap();
                    is_verified
                }
                Err(e) => {
                    eprintln!("{:?}", e);
                    false
                }
            };
            if !is_token_verified {
                eprintln!("Token is invalid");
            }

            println!("Token Verified");
            token = content;
            token.to_string()
        }

        Err(_) => {
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
                let is_token_verified = match super::api::verify_token(trimmed_token).await {
                    Ok(data) => {
                        let is_verified: bool = data["success"].to_string().parse().unwrap();
                        is_verified
                    }
                    Err(e) => {
                        eprintln!("{:?}", e);
                        continue;
                    }
                };

                if !is_token_verified {
                    eprintln!("Invalid Token Entered");
                    continue;
                }

                let _ = fs::write(&token_file_path, trimmed_token);
                token = trimmed_token.to_string().parse().unwrap();
            }
            token.to_string()
        }
    };
    token
}
