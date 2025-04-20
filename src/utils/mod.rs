use std::{fs, io, path::Path};

use actix_multipart::form::tempfile::TempFile;

pub mod models;

pub async fn save_uploaded_file(temp_file: &TempFile, target_path: &str) -> io::Result<()> {
    let tmp_path = temp_file.file.path();

    let destination = Path::new(target_path);

    if let Some(parent_dir) = destination.parent() {
        fs::create_dir_all(parent_dir)?;
    }

    fs::copy(tmp_path, destination)?;

    Ok(())
}