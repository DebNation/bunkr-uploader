use colored::*;
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::Client;
use reqwest::header::HeaderValue;
use reqwest::multipart::{Form, Part};
use serde::Deserialize;
use std::cmp::min;
use std::collections::HashMap;
use std::env;
use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::str;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct FinalResponse {
    pub files: Vec<Files>,
}

#[derive(Debug, Deserialize)]
pub struct Files {
    pub url: String,
}

pub struct FileInfo {
    pub name: String,
    pub size: u64,
    pub mime_type: String,
}

pub async fn process_uploads(
    files_paths: &[PathBuf],
    force_upload: bool,
    chunks_folder: &str,
    upload_url: &str,
    token: &str,
    album_id: &str,
    logs_file_path: &str,
    logs_file_writer: &mut BufWriter<File>,
) -> (Vec<String>, usize) {
    let mut uploads_direct_urls = vec![];
    let mut skipped_files_count = 0;

    for file_path in files_paths {
        let current_dir = env::current_dir().unwrap();
        let absolute_file_path = current_dir.join(file_path);
        let file_info = get_file_info(file_path);

        let logs_contents = fs::read_to_string(logs_file_path).unwrap_or_default();
        let file_path_string = file_path.to_string_lossy().to_string();

        if logs_contents.contains(&file_path_string) && !force_upload {
            skipped_files_count += 1;
            eprintln!(
                "{} Skipped, due to file already has been uploaded.",
                file_info.name
            );
            continue;
        }

        if file_info.size > 2_000_000_000 {
            eprintln!("Failed to upload '{}', is more than 2GB", file_info.name);
            continue;
        }

        let uuid_str = Uuid::new_v4().to_string();
        let chunk_size = 25_000_000;

        if file_info.size < chunk_size {
            let _ = upload_file(
                upload_url.to_owned(),
                token.to_owned(),
                file_info,
                album_id.to_owned(),
                absolute_file_path,
                &mut uploads_direct_urls,
                logs_file_writer,
            )
            .await;
        } else {
            let total_chunks = make_file_chunks(&absolute_file_path, chunks_folder, chunk_size);
            let _ = upload_big_file(
                chunks_folder,
                upload_url,
                token,
                &uuid_str,
                file_info,
                total_chunks,
                chunk_size,
                album_id,
                &mut uploads_direct_urls,
                absolute_file_path,
                logs_file_writer,
            )
            .await;
        }
    }
    (uploads_direct_urls, skipped_files_count)
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
    absolute_file_path: PathBuf,
    logs_file_writer: &mut BufWriter<File>,
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
        let file_part = Part::bytes(file_contents)
            .file_name(file_info.name.clone())
            .mime_str(&file_info.mime_type)?;

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
                "✗ Upload failed for chunk {}: Status Code: {} - Response: {}",
                chunk_index, status, body
            );
            continue;
        }
        let new_progrss = min(uploaded + chunk_size, file_info.size);
        uploaded = new_progrss;
        pb.set_position(new_progrss.into());
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

    if rebuild_file_res.status() == 500 {
        eprintln!(
            "{}",
            "You have been rate limited, Please try again after sometime".bold()
        );
    }
    let res_body = rebuild_file_res.text().await.unwrap();

    let data: FinalResponse = serde_json::from_str(&res_body)?;
    println!("{} ✔ ", file_info.name);
    uploads_direct_urls.push(data.files[0].url.to_string());

    let full_path_string = absolute_file_path.to_string_lossy();
    writeln!(logs_file_writer, "{}", full_path_string)?;
    writeln!(logs_file_writer, "{}", file_info.name)?;
    logs_file_writer.flush().unwrap();

    Ok(())
}

async fn upload_file(
    upload_url: String,
    token: String,
    file_info: FileInfo,
    album_id: String,
    full_path: PathBuf,
    uploads_direct_urls: &mut Vec<String>,
    logs_file_writer: &mut BufWriter<File>,
) -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();

    let file_contents = match fs::read(&full_path) {
        Ok(contents) => contents,
        Err(e) => {
            println!("✗ Failed to read {}", e);
            Err(e)?
        }
    };

    let file_part = Part::bytes(file_contents)
        .file_name(file_info.name.clone())
        .mime_str(&file_info.mime_type)?;
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
    if res.status() != 200 {
        eprintln!("Failed to upload,  {:?}", &res);
    }

    if res.status() == 500 {
        eprintln!(
            "{}",
            "You have been rate limited, Please try again after sometime.".bold()
        );
    }
    let json_data: FinalResponse = serde_json::from_str(&res.text().await.unwrap())?;

    let url = json_data.files[0].url.to_string();

    if !url.is_empty() {
        uploads_direct_urls.push(url);
    }

    let full_path_string = full_path.to_string_lossy();
    writeln!(logs_file_writer, "{}", full_path_string)?;
    writeln!(logs_file_writer, "{}", file_info.name)?;
    logs_file_writer.flush().unwrap();

    println!("{} ✔ ", file_info.name);
    Ok(())
}

//INFO: OPTIONAL_HEADERS:
// .header("accept", "application/json")
// .header("origin", "https://dash.bunkr.cr")
// .header("referer", "https://dash.bunkr.cr/")
// .header("x-requested-with", "XMLHttpRequest")
// .header(
//     "user-agent",
//     "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 \
//  (KHTML, like Gecko) Chrome/152.0.0.0 Safari/537.36",
// );
