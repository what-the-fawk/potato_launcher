use futures::StreamExt as _;
use reqwest::Client;
use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::process::Stdio;
use std::sync::Arc;

use crate::config::build_config;
use crate::lang::LangMessage;
use crate::progress::ProgressBar;

lazy_static::lazy_static! {
    static ref VERSION_URL: String = format!("{}/launcher/version.txt", build_config::get_server_base());
}

#[cfg(target_os = "windows")]
lazy_static::lazy_static! {
    static ref LAUNCHER_BINARY_NAME: String = format!("{}.exe", build_config::get_launcher_name());
}
#[cfg(target_os = "linux")]
lazy_static::lazy_static! {
    static ref LAUNCHER_BINARY_NAME: String = format!("{}_linux", build_config::get_launcher_name());
}
#[cfg(target_os = "macos")]
lazy_static::lazy_static! {
    static ref LAUNCHER_BINARY_NAME: String = format!("{}_macos", build_config::get_launcher_name());
}

lazy_static::lazy_static! {
    static ref UPDATE_URL: String = format!("{}/launcher/{}", build_config::get_server_base(), *LAUNCHER_BINARY_NAME);
}

async fn fetch_new_version() -> Result<String, Box<dyn std::error::Error>> {
    let client = Client::new();
    let response = client.get(&*VERSION_URL).send().await?.error_for_status()?;
    let text = response.text().await?;
    Ok(text.trim().to_string())
}

pub async fn need_update() -> Result<bool, Box<dyn std::error::Error>> {
    let new_version = fetch_new_version().await?;
    let current_version = build_config::get_version().expect("Version not set");
    Ok(new_version != current_version)
}

pub async fn fetch_new_binary(
    progress_bar: Arc<dyn ProgressBar + Send + Sync>,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let client = Client::new();
    let response = client
        .get(UPDATE_URL.as_str())
        .send()
        .await?
        .error_for_status()?;

    let total_size = response.content_length().unwrap_or(0);
    progress_bar.set_length(total_size);
    progress_bar.set_message(LangMessage::DownloadingUpdate);

    let mut bytes = Vec::with_capacity(total_size as usize);
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        bytes.extend_from_slice(&chunk);
        progress_bar.inc(chunk.len() as u64);
    }
    progress_bar.finish();

    Ok(bytes)
}

#[cfg(not(target_os = "windows"))]
fn replace_binary(current_path: &Path, new_binary: &[u8]) -> std::io::Result<()> {
    use super::compat::chmod_x;

    fs::write(current_path, new_binary)?;
    chmod_x(current_path);
    Ok(())
}

#[cfg(target_os = "windows")]
fn replace_binary(current_path: &Path, new_binary: &[u8]) -> std::io::Result<()> {
    use crate::utils;

    let temp_path = utils::get_temp_dir().join("update.exe");
    fs::write(&temp_path, new_binary)?;
    Command::new("cmd")
        .args(&[
            "/C",
            "move",
            "/Y",
            temp_path.to_str().unwrap(),
            current_path.to_str().unwrap(),
        ])
        .spawn()?;
    Ok(())
}

pub fn replace_binary_and_launch(new_binary: &[u8]) -> std::io::Result<()> {
    let current_exe = env::current_exe()?;
    replace_binary(&current_exe, &new_binary)?;
    let args: Vec<String> = env::args().collect();
    let mut new_process = Command::new(&current_exe)
        .args(&args[1..])
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()?;
    let result = new_process.wait()?;
    std::process::exit(result.code().unwrap_or(1));
}
