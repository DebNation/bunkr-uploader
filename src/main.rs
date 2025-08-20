use clap::Parser;
use colored::*;
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::Client;
use reqwest::header::HeaderValue;
use reqwest::multipart::{Form, Part};
use serde::Deserialize;
use std::cmp::min;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, ErrorKind, Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::{
    env,
    fs::{self},
    io::{self},
};
use uuid::Uuid;

use crate::utils::extras::handle_paths;
mod utils;

#[derive(Debug, Deserialize)]
pub struct FinalResponse {
    pub success: bool,
    pub files: Vec<Files>,
}

#[derive(Debug, Deserialize)]
pub struct Files {
    pub name: String,
    pub url: String,
}

pub struct FileInfo {
    pub name: String,
    pub size: u64,
    pub mime_type: String,
}

#[derive(Parser)]
pub struct Args {
    #[arg(short = 'u', long = "upload", help = "path to directory or file")]
    upload: String,
}

#[tokio::main]
async fn main() {
    human_panic::setup_panic!();
    let path = Args::parse().upload;
    let mut files_paths: Vec<PathBuf> = vec![];
    let mut upload_from_all_sub_dir = false;
    handle_paths(path, &mut files_paths, &mut upload_from_all_sub_dir);
    // match fs::read_dir(&path) {
    //     Ok(entries) => {
    //         for entry in entries {
    //             match entry {
    //                 Ok(entry) => {
    //                     let file_path = entry.path();
    //                     let metadata = file_path
    //                         .metadata()
    //                         .expect(&format!("failed to detect metadata of {:?}", file_path));
    //                     if metadata.is_file() {
    //                         files_paths.push(file_path.to_owned());
    //                     }
    //                 }
    //                 Err(e) => {
    //                     panic!("{}", e);
    //                 }
    //             }
    //         }
    //     }
    //     Err(e) => {
    //         panic!("{}", e);
    //         // if e.kind() == ErrorKind::NotADirectory {
    //         //     // files_paths.push(path.into());
    //         // } else {
    //         // }
    //     }
    // }

    println!("You are uploading: ");
    for path in &files_paths {
        println!("{:?}", path);
    }

    let home = env::var("HOME").expect("HOME is not set");
    let token_dir = format!("{}/.local/share/bunkr-uploader", home);
    let token_file_path = format!("{}/token.txt", &token_dir);
    let chunks_folder = format!("{}/chunks", token_dir);
    fs::create_dir_all(&chunks_folder).expect("failed to create chunks directory");
    let token: String = utils::extras::handle_token(token_file_path).await;

    let upload_url: String = match utils::api::get_data(&token).await {
        Ok(data) => {
            let url: String = data["url"].to_string().parse().unwrap();
            let clean_url = url.trim_matches('"');
            clean_url.to_owned()
        }

        Err(e) => {
            println!("Error: {}", e);
            return;
        }
    };

    println!("Add to your album ? y/n");

    let mut upload_to_album: String = String::new();
    io::stdin().read_line(&mut upload_to_album).unwrap();

    let mut album_id: String = String::new();
    if upload_to_album.trim() == "y"
        || upload_to_album.trim() == "Y"
        || upload_to_album.trim() == ""
    {
        match utils::api::get_albums(&token).await {
            Ok(data) => {
                let labels: Vec<String> = data
                    .albums
                    .iter()
                    .map(|album| format!("{} (id: {})", album.name, album.id))
                    .collect();
                let selection = dialoguer::Select::new()
                    .with_prompt("Select an Album")
                    .items(&labels)
                    .default(0)
                    .interact()
                    .unwrap();
                album_id = data.albums[selection].id.to_string();
            }
            Err(err) => eprintln!("Error getting albums{}", err),
        };
    }
    let mut uploads_direct_urls: Vec<String> = vec![];
    for file_path in files_paths {
        let current_dir = env::current_dir().unwrap();
        let absolute_file_path = current_dir.join(&file_path);
        let file_info = get_file_info(&file_path);

        if file_info.size > 2000 * 1000 * 1000 {
            eprintln!("Failed to upload '{}', is more than 2GB", file_info.name);
            continue;
        }
        let uuid = Uuid::new_v4();
        let uuid_str = uuid.to_string();

        let chunk_size: u64 = 25 * 1000 * 1000;
        if file_info.size < chunk_size {
            let _ = upload_file(
                upload_url.to_owned(),
                token.to_owned(),
                file_info,
                album_id.to_owned(),
                absolute_file_path.to_owned(),
                &mut uploads_direct_urls,
            )
            .await;
        } else {
            let total_chunks: u8 =
                make_file_chunks(&absolute_file_path, &chunks_folder, chunk_size);
            let _ = upload_big_file(
                &chunks_folder,
                &upload_url,
                &token,
                &uuid_str,
                file_info,
                total_chunks,
                chunk_size,
                &album_id,
                &mut uploads_direct_urls,
            )
            .await;
        }
    }
    for (index, url) in uploads_direct_urls.iter().enumerate() {
        println!("{}: {}", index + 1, url.yellow());
    }
}

fn get_file_info(file_path: &PathBuf) -> FileInfo {
    let basename = Command::new("basename")
        .arg(&file_path)
        .output()
        .expect("basename command failed to start");
    let name = str::from_utf8(&basename.stdout).unwrap().trim().to_string();

    let get_size = Command::new("stat")
        .arg("-c%s")
        .arg(&file_path)
        .output()
        .expect("size command failed to start");
    let str_size = str::from_utf8(&get_size.stdout).unwrap().trim();
    let size: u64 = str_size.parse().unwrap();

    let mimetype_stdout = Command::new("file")
        .arg("--mime-type")
        .arg("-b")
        .arg(&file_path)
        .output()
        .expect("size command failed to start");
    let mime_type = str::from_utf8(&mimetype_stdout.stdout)
        .expect("invalid UTF-8 in `file` output")
        .trim()
        .to_string();

    FileInfo {
        name,
        size,
        mime_type,
    }
}

fn make_file_chunks(file: &PathBuf, chunks_folder: &str, chunk_size: u64) -> u8 {
    let input_file = File::open(&file).unwrap();
    let mut reader = BufReader::new(input_file);
    // let chunk_size_usize: u64 = chunk_size.try_into().unwrap();
    let mut buffer = vec![0u8; chunk_size.try_into().unwrap()];
    let mut chunk_index = 0;
    loop {
        let bytes_read = reader.read(&mut buffer).unwrap();
        if bytes_read == 0 {
            break;
        }

        let chunk_filename = format!("chunk_{}", chunk_index);
        let chunk_path = Path::new(chunks_folder).join(&chunk_filename);
        let mut chunk_file = File::create(&chunk_path).unwrap();
        chunk_file.write_all(&buffer[..bytes_read]).unwrap();
        chunk_index += 1;
    }
    return chunk_index;
}

async fn upload_big_file(
    chunks_folder: &str,
    upload_url: &str,
    token: &str,
    uuid: &str,
    file_info: FileInfo,
    total_chunks: u8,
    chunk_size: u64,
    album_id: &str,
    uploads_direct_urls: &mut Vec<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();
    let mut uploaded = 0;
    let total_size = file_info.size;
    let pb = ProgressBar::new(total_size.into());
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{msg} [{bar:40.green/black}] {bytes}/{total_bytes} {percent}%")
            .unwrap()
            .progress_chars("=>-"),
    );
    pb.set_message(format!("{}", file_info.name));

    for chunk_index in 0..total_chunks {
        let chunk_filename = format!("chunk_{}", chunk_index);
        let chunk_index_path = PathBuf::from(chunks_folder).join(&chunk_filename);
        if !chunk_index_path.exists() {
            println!("✗ Chunk file {} does not exist, skipping", chunk_filename);
            continue;
        }
        let file_contents = match fs::read(&chunk_index_path) {
            Ok(contents) => contents,
            Err(e) => {
                println!("✗ Failed to read {}: {}", chunk_filename, e);
                continue;
            }
        };

        let byte_offset = chunk_index as u64 * chunk_size as u64;
        let file_part = Part::bytes(file_contents).file_name(chunk_filename);

        let form = Form::new()
            .text("dzuuid", uuid.to_string())
            .text("dzchunkindex", chunk_index.to_string())
            .text("dztotalfilesize", file_info.size.to_string())
            .text("dzchunksize", chunk_size.to_string())
            .text("dztotalchunkcount", total_chunks.to_string())
            .text("dzchunkbyteoffset", byte_offset.to_string())
            .part("files[]", file_part);

        let request = client
            .post(upload_url)
            .header("token", HeaderValue::from_str(&token)?);
        let request_with_form = request.multipart(form);
        let res = match request_with_form.send().await {
            Ok(response) => response,
            Err(e) => {
                println!(
                    "✗ Network error while uploading chunk {}: {}",
                    chunk_index, e
                );
                continue;
            }
        };

        if !res.status().is_success() {
            let status = res.status();
            let body = res
                .text()
                .await
                .unwrap_or_else(|_| "Could not read response".to_string());
            println!(
                "✗ Upload failed for chunk {}: {} - Response: {}",
                chunk_index, status, body
            );
            continue;
        }
        let new = min(uploaded + chunk_size, file_info.size);
        uploaded = new;
        pb.set_position(new.into());
        fs::remove_file(chunk_index_path).unwrap();
    }

    let mut map: HashMap<&str, String> = HashMap::new();
    map.insert("uuid", uuid.to_string());
    map.insert("original", file_info.name.to_string());
    map.insert("type", file_info.mime_type.to_string());
    if !album_id.is_empty() {
        map.insert("albumid", album_id.to_string());
    }
    map.insert("age", "null".to_string());
    map.insert("filelength", "null".to_string());
    let finish_chunk_endpoint = format!("{}/finishchunks", upload_url);

    let mut final_payload = HashMap::new();
    final_payload.insert("files", [map]);

    let rebuild_file_res = client
        .post(&finish_chunk_endpoint)
        .header("token", HeaderValue::from_str(&token)?)
        .json(&final_payload)
        .send()
        .await
        .unwrap();
    if !rebuild_file_res.status().is_success() {
        eprintln!("Failed to Upload the file, {:?}", rebuild_file_res);
    }
    let res_body = rebuild_file_res.text().await.unwrap();

    let data: FinalResponse = serde_json::from_str(&res_body)?;
    if !data.success {
        eprintln!("Failed to upload: {:?}", data);
    }
    println!("{} ✔ ", file_info.name);
    uploads_direct_urls.push(data.files[0].url.to_string());

    Ok(())
}

async fn upload_file(
    upload_url: String,
    token: String,
    file_info: FileInfo,
    album_id: String,
    full_path: PathBuf,
    uploads_direct_urls: &mut Vec<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();

    let file_contents = match fs::read(&full_path) {
        Ok(contents) => contents,
        Err(e) => {
            println!("✗ Failed to read {}", e);
            Err(e)?
        }
    };

    let file_part = Part::bytes(file_contents).file_name(file_info.name.clone());
    let form = Form::new().part("files[]", file_part);

    let request = client
        .post(&upload_url)
        .header("token", HeaderValue::from_str(&token)?)
        .header("albumid", HeaderValue::from_str(&album_id)?);
    let request_with_form = request.multipart(form);
    let res = match request_with_form.send().await {
        Ok(response) => response,
        Err(e) => {
            println!("✗ Network error while uploading chunk : {}", e);
            Err(e)?
        }
    };
    let json_data: FinalResponse = serde_json::from_str(&res.text().await.unwrap())?;
    if !json_data.success {
        eprintln!("Failed to upload: {:?}", json_data);
    }
    let url = json_data.files[0].url.to_string();
    // println!("Upload URL: {}", json_data.files[0].url.yellow().bold());

    if !url.is_empty() {
        uploads_direct_urls.push(url);
    }
    println!("{} ✔ ", file_info.name);
    Ok(())
}
