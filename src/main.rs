use clap::Parser;
use colored::*;
use rand::RngExt;
use rand::distr::Alphanumeric;
use std::env;
use std::fs::{self, OpenOptions};
use std::io::BufWriter;

mod modules;
mod utils;

#[derive(Parser)]
#[command(version, about)]
pub struct Args {
    #[arg(help = "path to files or directory")]
    paths: Vec<String>,

    #[arg(short = 'f', help = "force upload without skipping(for special case)")]
    force: bool,
}

#[tokio::main]
async fn main() {
    human_panic::setup_panic!();
    let args = Args::parse();
    let paths = args.paths;
    let force_upload = args.force;

    let home = env::var("HOME").expect("HOME is not set");
    let resources_path = format!("{}/.local/share/bunkr-uploader", home);

    let files_paths = utils::paths::get_file_paths(paths, &resources_path);
    if files_paths.is_empty() {
        return;
    }

    println!("You are uploading: ");
    for path in &files_paths {
        println!("{}", path.to_string_lossy());
    }
    println!(
        "{}",
        format!("Total files: {}", files_paths.len())
            .yellow()
            .bold()
    );

    let token_file_path = format!("{}/token.txt", &resources_path);
    let logs_file_path = format!("{}/logs.txt", &resources_path);
    let random_string: String = rand::rng()
        .sample_iter(&Alphanumeric)
        .take(10)
        .map(char::from)
        .collect();

    fs::create_dir_all(&resources_path).expect("failed to create resources path");
    utils::fs::delete_all_dir(&resources_path);

    let chunks_folder = format!("{}/{}", &resources_path, &random_string);
    fs::create_dir_all(&chunks_folder).expect("failed to create chunks directory");

    let logs_file = OpenOptions::new()
        .append(true)
        .create(true)
        .open(&logs_file_path)
        .unwrap();
    let mut logs_file_writer = BufWriter::new(logs_file);

    let token: String = utils::token::handle_token(token_file_path).await;
    let upload_url: String = match utils::api::get_data(&token).await {
        Ok(data) => {
            let url: String = data["url"].to_string().parse().unwrap();
            url.trim_matches('"').to_owned()
        }
        Err(e) => {
            println!("Error: {}", e);
            return;
        }
    };

    let album_id = modules::album::get_or_create_album(&token).await;

    let (uploads_direct_urls, skipped_count) = modules::upload::process_uploads(
        &files_paths,
        force_upload,
        &chunks_folder,
        &upload_url,
        &token,
        &album_id,
        &logs_file_path,
        &mut logs_file_writer,
    )
    .await;

    for (index, url) in uploads_direct_urls.iter().enumerate() {
        println!("{}: {}", index + 1, url.yellow());
    }

    println!(
        "{}",
        format!("Success: {}", uploads_direct_urls.len())
            .green()
            .bold()
    );

    println!(
        "{}",
        format!(
            "Failed: {}",
            files_paths.len() - (uploads_direct_urls.len() + skipped_count)
        )
        .red()
        .bold()
    );

    let _ = fs::remove_dir_all(chunks_folder);
}
