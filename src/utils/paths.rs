use std::io::ErrorKind;
use std::path::PathBuf;
use std::process::Command;
use std::{fs, io};

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
                        eprintln!("{}", e);
                        return;
                    }
                }
            }
        }
        Err(e) => {
            if e.kind() == ErrorKind::NotADirectory {
                files_paths.push(path.into());
            } else {
                eprintln!("{}", e);
                return;
            }
        }
    }
}

pub fn get_file_paths(paths: Vec<String>, resources_path: &str) -> Vec<PathBuf> {
    let mut files_paths: Vec<PathBuf> = vec![];
    let mut upload_from_all_sub_dir = false;
    for path in paths {
        handle_paths(path, &mut files_paths, &mut upload_from_all_sub_dir);
    }

    if files_paths.is_empty() {
        let picked_files = format!("{}/yazi-picked.txt", resources_path);
        let yazi_exists = Command::new("yazi").arg("--version").output().is_ok();

        if yazi_exists {
            let _ = Command::new("yazi")
                .arg("--chooser-file")
                .arg(&picked_files)
                .status();

            if let Ok(content) = fs::read_to_string(&picked_files) {
                for file in content.lines() {
                    handle_paths(
                        file.to_string(),
                        &mut files_paths,
                        &mut upload_from_all_sub_dir,
                    );
                }
            } else {
                eprintln!("No files selected.");
            }

            let _ = fs::remove_file(&picked_files);
        } else {
            eprintln!("yazi is not installed, please manually pass the file paths");
        }
    }
    files_paths
}
