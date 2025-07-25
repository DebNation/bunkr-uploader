use indicatif::{ProgressBar, ProgressStyle};
use reqwest::Client;
use reqwest::header::HeaderValue;
use reqwest::multipart::{Form, Part};
use std::cmp::min;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::{
    env,
    fs::{self},
    io::{self},
};
use uuid::Uuid;

mod utils;

#[tokio::main]
async fn main() {
    let home = env::var("HOME").expect("HOME is not set");
    let token_dir = format!("{}/.local/share/bunkr-uploader", home);
    let _ = fs::create_dir_all(&token_dir).expect("failed to create directory");

    let token_file_path = format!("{}/token.txt", &token_dir);
    let chunks_folder = format!("{}/chunks", token_dir);
    std::fs::create_dir_all(&chunks_folder).unwrap();
    let token = match fs::read_to_string(&token_file_path) {
        Ok(content) => content.trim().to_string(),

        Err(_) => {
            io::stdout().flush().unwrap();
            println!("Enter token: ");
            let mut input_token: String = String::new();
            io::stdin().read_line(&mut input_token).unwrap();
            let trimmed_content = input_token.trim();
            if trimmed_content.is_empty() {
                // Err("Token can't be empty").unwrap();
                panic!("Token can't be empty")
            }
            let _ = fs::write(&token_file_path, input_token);
            return ();
        }
    };

    let args: Vec<String> = env::args().collect();
    if !fs::exists(&args[1]).unwrap() {
        panic!("File/Folder is not found");
    }

    let upload_url: String = match utils::get_data(&token).await {
        Ok(data) => {
            let url: String = data["url"].to_string();
            url
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
    if upload_to_album.trim() == "y" || upload_to_album.trim() == "Y" {
        println!("Enter album id:");
        io::stdin().read_line(&mut album_id).unwrap();
    }
    let current_dir = env::current_dir().unwrap();
    let full_path = current_dir.join(&args[1]);
    let file_info = get_file_info(&full_path);
    let uuid = Uuid::new_v4();
    let uuid_str = uuid.to_string();

    let chunk_size: u32 = 25 * 1000 * 1000;
    let total_chunks: u8 = make_file_chunks(&full_path, &chunks_folder, chunk_size);
    let _ = upload_file(
        &chunks_folder,
        upload_url,
        token,
        &uuid_str,
        file_info,
        total_chunks,
        chunk_size,
        album_id,
    )
    .await;
}

fn get_file_info(file_path: &PathBuf) -> (String, u32, String) {
    let basename = Command::new("basename")
        .arg(&file_path)
        .output()
        .expect("basename command failed to start");
    let str_basename = str::from_utf8(&basename.stdout).unwrap().trim();

    let size = Command::new("stat")
        .arg("-c%s")
        .arg(&file_path)
        .output()
        .expect("size command failed to start");
    let str_size = str::from_utf8(&size.stdout).unwrap().trim();
    let int_size: u32 = str_size.parse().unwrap();

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

    return (str_basename.to_string(), int_size, mime_type);
}

fn make_file_chunks(file: &PathBuf, chunks_folder: &str, chunk_size: u32) -> u8 {
    let input_file = File::open(&file).unwrap();
    let mut reader = BufReader::new(input_file);
    let chunk_size_usize: usize = chunk_size.try_into().unwrap();
    let mut buffer = vec![0u8; chunk_size_usize];
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

async fn upload_file(
    chunks_folder: &str,
    upload_url: String,
    token: String,
    uuid: &str,
    file_info: (String, u32, String),
    total_chunks: u8,
    chunk_size: u32,
    album_id: String,
) -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();
    let mut uploaded = 0;
    let total_size = file_info.1;
    let pb = ProgressBar::new(total_size.into());
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{msg} [{bar:80.green/black}] {bytes}/{total_bytes} {percent}%")
            .unwrap()
            .progress_chars("=>-"),
    );
    pb.set_message(format!("{}", file_info.0));

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
        let file_part = Part::bytes(file_contents).file_name(chunk_filename.clone());
        let form = Form::new()
            .text("dzuuid", uuid.to_string())
            .text("dzchunkindex", chunk_index.to_string())
            .text("dztotalfilesize", file_info.1.to_string())
            .text("dzchunksize", chunk_size.to_string())
            .text("dztotalchunkcount", total_chunks.to_string())
            .text("dzchunkbyteoffset", byte_offset.to_string())
            .part("files[]", file_part);
        let request = client
            .post(&upload_url)
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
        let new = min(uploaded + chunk_size, file_info.1);
        uploaded = new;
        pb.set_position(new.into());
    }

    let mut map: HashMap<&str, String> = HashMap::new();
    map.insert("uuid", uuid.to_string());
    map.insert("original", file_info.0.to_string());
    map.insert("type", file_info.2.to_string());
    if !album_id.is_empty() {
        map.insert("albumid", album_id);
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
        eprintln!("{:?}", rebuild_file_res);
    }
    let res_body = rebuild_file_res.text().await.unwrap();
    println!("{}", res_body);

    println!("Upload Done");
    fs::remove_dir_all(chunks_folder).unwrap();
    Ok(())
}
