use base64::{Engine as _, engine::general_purpose};
use std::io::{ErrorKind, Write};
use std::{
    fs::{self},
    io::{self},
};
async fn get_actual_token(token: &str, token_file_path: &str) -> String {
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
            new_token = token.to_string();
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

        Err(err) => match err.kind() {
            ErrorKind::NotFound => get_actual_token("", &token_file_path).await,
            _ => return Err(err).expect("Failed to parse token file"),
        },
    };
    let verified_token = get_actual_token(&token, &token_file_path).await;
    // println!("Token Verified");
    verified_token
}

pub fn handle_paths(
    path: String,
    mut files_paths: &mut Vec<std::path::PathBuf>,
    upload_from_all_sub_dir: &mut bool,
) {
    match fs::read_dir(&path) {
        Ok(entries) => {
            for entry in entries {
                match entry {
                    Ok(entry) => {
                        let file_path = entry.path();
                        let metadata = file_path
                            .metadata()
                            .expect(&format!("failed to detect metadata of {:?}", file_path));
                        if !metadata.is_file() {
                            let mut upload_from_sub_dir: String = String::new();
                            if *upload_from_all_sub_dir == false {
                                println!(
                                    "{}",
                                    format!(
                                        "Do you want to upload from this subdir {:?}, y/n/A",
                                        file_path
                                    )
                                );
                                io::stdin().read_line(&mut upload_from_sub_dir).unwrap();
                            }
                            if upload_from_sub_dir.trim() == "A" {
                                *upload_from_all_sub_dir = true;
                            }

                            if upload_from_sub_dir.trim() == "Y"
                                || upload_from_sub_dir.trim() == "y"
                                || upload_from_sub_dir.trim() == ""
                                || upload_from_sub_dir.trim() == "A"
                            {
                                handle_paths(
                                    file_path.to_string_lossy().to_string(),
                                    &mut files_paths,
                                    upload_from_all_sub_dir,
                                );
                            }
                        } else {
                            files_paths.push(file_path.to_owned());
                        }
                    }
                    Err(e) => {
                        panic!("{}", e);
                    }
                }
            }
        }
        Err(e) => {
            if e.kind() == ErrorKind::NotADirectory {
                files_paths.push(path.into());
            } else {
                panic!("{}", e);
            }
        }
    }
}
