use std::{
    path::{Path, PathBuf},
    process::{Output, Stdio},
};

use tokio::process::Command;

use crate::types::{BotError, BotErrorKind, BotResult};

pub async fn download_audio(url: &String) -> BotResult<(String, PathBuf)> {
    // Don't download yet, only get the title of video to use later
    let dry_run_args = vec!["--simulate", "--print", "%(channel)s - %(title)s"];
    let dry_run_output = run_yt_dlp(&url, dry_run_args).await?;
    // The string from stdout has a newline at the end we don't want
    let file_title = String::from_utf8(dry_run_output.stdout)?.replace("\n", "");

    // Download the video using the video ID as the filename
    let download_args = vec![
        "--no-simulate",
        "--verbose",
        "--print",
        "after_move:filepath",
    ];
    let download_output = run_yt_dlp(&url, download_args).await?;
    // The string from stdout has a newline at the end we don't want
    let file_path_string = String::from_utf8(download_output.stdout)?.replace("\n", "");
    let file_path = PathBuf::from(file_path_string);

    Ok((file_title, file_path))
}

async fn run_yt_dlp(url: &String, custom_args: Vec<&str>) -> BotResult<Output> {
    let yt_dlp_path = Path::new("yt-dlp");
    let download_path = Path::new("/tmp/.cache");
    let default_args = vec![
        "--quiet",
        "--no-warnings",
        "--format",
        "bestaudio",
        "--extract-audio",
        "--audio-format",
        "m4a",
        "--add-metadata",
        // TODO: Figure out why this works in local container and fails in fly.io container
        //"--embed-thumbnail",
        "--output",
        "%(id)s.%(ext)s",
    ];
    let mut command = Command::new(yt_dlp_path);
    command
        .current_dir(download_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for arg in default_args.into_iter() {
        command.arg(arg);
    }
    for arg in custom_args.into_iter() {
        command.arg(arg);
    }
    // Make sure the source url is the last argument
    command.arg(url);

    match command.spawn()?.wait_with_output().await {
        Ok(output) => Ok(output),
        Err(_) => Err(BotError::new(BotErrorKind::DownloadError)),
        // panic!(
        //     "downloading video failed:\nstdout: {}\nstderr: {}",
        //     String::from_utf8(output.stdout)?,
        //     String::from_utf8(output.stderr)?
        // );
    }
}
