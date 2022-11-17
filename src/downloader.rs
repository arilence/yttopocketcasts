use std::{
    path::{Path, PathBuf},
    process::{Output, Stdio},
};

use tokio::process::Command;

pub async fn download_audio(url: &String) -> PathBuf {
    let args = vec![
        "--quiet",
        "--no-warnings",
        "--print",
        "after_move:filepath",
        "--no-simulate",
        "--format",
        "bestaudio",
        "--extract-audio",
        "--audio-format",
        "m4a",
        "--add-metadata",
        "--embed-thumbnail",
        // NOTE: Keep special characters for now. This might change.
        // "--restrict-filenames",
        "--output",
        "%(channel)s - %(title)s.%(ext)s",
    ];
    let output = run_yt_dlp(&url, args).await;
    if !output.status.success() {
        panic!("yt-dlp failed");
    }
    let file_path = String::from_utf8(output.stdout).expect("Failed to parse stdout as Utf8");
    // The string from stdout has a newline at the end we don't want part of the PathBuf
    let path = PathBuf::from(file_path.replace("\n", ""));
    return path;
}

async fn run_yt_dlp(url: &String, args: Vec<&str>) -> Output {
    let yt_dlp_path = Path::new("yt-dlp");
    let download_path = Path::new(".cache");
    let mut command = Command::new(yt_dlp_path);
    command
        .current_dir(download_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    for arg in args.into_iter() {
        command.arg(arg);
    }
    // Make sure the source url is the last argument
    command.arg(url);

    let output = command
        .spawn()
        .expect("yt-dlp failed to start")
        .wait_with_output()
        .await
        .expect("yt-dlp failed to run");
    return output;
}
