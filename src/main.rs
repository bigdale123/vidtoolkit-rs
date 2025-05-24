use clap::{Parser};
use std::fs;
use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;
use indicatif::ProgressBar;

#[derive(Parser)]
#[command(name = "vidconvert-rs")]
#[command(version = "1.0")]
#[command(about = "Converts non H264 video files into H264 video files using a handbrake preset.", long_about = None)]
struct Cli {
    /// Path(s) to run vidconvert-rs on
    paths: Vec<String>,

    /// Turn debugging information on
    #[arg(long)]
    debug: bool,

    /// Turn debugging information on
    #[arg(long)]
    dry_run: bool,

    /// Turn debugging information on
    #[arg(long)]
    include_h264: bool,

    /// Do not perform any transcoding (useful if you just want to generate subtitles)
    #[arg(long)]
    no_transcode: bool,

    /// Generate Subtitles using Whisper for all videos that do no contain subtitles
    #[arg(long)]
    gen_subs: bool
}

fn check_for_h264(video: &Path) -> bool {
    let ffprobe_command = Command::new("ffprobe")
        .arg("-v")
        .arg("error")
        .arg("-select_streams")
        .arg("v:0")
        .arg("-show_entries")
        .arg("stream=codec_name")
        .arg("-of")
        .arg("default=noprint_wrappers=1:nokey=1")
        .arg(video)
        .output();
    if ffprobe_command.as_ref().expect("No Output from Command.").stdout.len() > 0 {
        let output = ffprobe_command.expect("No Output from Command.").stdout.clone();
        let codec_name = String::from_utf8_lossy(&output);
        return codec_name.trim() == "h264"
    }
    else {
        return false;
    }
    
}

fn check_for_subs(video: &Path) -> bool {
    let ffprobe_command = Command::new("ffprobe")
        .arg("-v")
        .arg("error")
        .arg("-select_streams")
        .arg("s")
        .arg("-show_entries")
        .arg("stream=index")
        .arg("-of")
        .arg("csv=p=0")
        .arg(video)
        .output();
    if ffprobe_command.as_ref().expect("No Output from Command.").stdout.len() > 0 {
        return true;
    }
    else {
        return false;
    }
    
}

fn convert_video(video: &Path, cli_parse: &Cli) {
    // println!("PLACEHOLDER: {}", video.display());
    let temp_file = video.with_file_name("temp_file.mkv");
    let new_file = video.with_file_name("new_file.mkv");
    let preset_file = env::current_dir().expect("ERROR | Could not get current working directory.").join("presets.json");
    let handbrake_command = Command::new("HandBrakeCLI")
        .arg("-i")
        .arg(video)
        .arg("-o")
        .arg(temp_file.clone())
        .arg("--preset-import-file")
        .arg(preset_file.clone())
        .arg("--preset")
        .arg("Fast 1080p NVENC")
        .output();
    if handbrake_command.as_ref().expect("ERROR | No Output from Command.").stdout.len() > 0 {
        if cli_parse.debug {
            println!("{}", String::from_utf8_lossy(&handbrake_command.as_ref().expect("ERROR | No Output from Command.").stdout.clone()));
        }
        let mkvmerge_command = Command::new("mkvmerge")
            .arg("-o")
            .arg(new_file.clone())
            .arg("-D")
            .arg("-A")
            .arg(video)
            .arg("-S")
            .arg("-B")
            .arg("-T")
            .arg("-M")
            .arg(temp_file.clone())
            .output();
        if mkvmerge_command.as_ref().expect("ERROR | No Output from Command.").stdout.len() > 0 {
            if cli_parse.debug {
                println!("{}", String::from_utf8_lossy(&mkvmerge_command.as_ref().expect("ERROR | No Output from Command.").stdout.clone()));
            }
            let _ = fs::rename(new_file.clone(), video);
            let _ = fs::remove_file(temp_file.clone());
        }
    }
}

fn get_videos(directory: &Path, cli_parse: &Cli) -> Vec<PathBuf> {
    let mut videos: Vec<PathBuf> = Vec::new();

    let valid_extension = [
        String::from("mp4"),
        String::from("mkv"),
        String::from("avi"),
        String::from("mov"),
        String::from("wmv"),
        String::from("flv"),
        String::from("webm"),
    ];

    if let Ok(files) = fs::read_dir(directory) {
        for file in files {
            if let Ok(file) = file {
                let path = file.path();
                // println!("{}", path.display());
                if path.is_dir() {
                    videos.extend(get_videos(&path, &cli_parse));
                }
                else if valid_extension.contains(&path.extension().expect(&format!("ERROR | No Extension found for file {}", &path.display())).to_string_lossy().to_lowercase()) {
                    if cli_parse.include_h264 {
                        videos.push(path.clone()); 
                    }
                    else if !check_for_h264(&path) {
                        videos.push(path.clone());
                    }
                }
            }
        }
    }
    return videos;
}

fn main() {
    let cli_parse = Cli::parse();
    for i in &cli_parse.paths {
        // println!("{}", i);
        let directory = Path::new(i);
        let videos = get_videos(directory, &cli_parse);
        if cli_parse.dry_run {
            println!("The Following files WILL be converted in path {}:",i);
            for video in &videos {
                println!("  {}", video.display());
            }
        }
        else {
            let pb = ProgressBar::new(videos.len().try_into().unwrap());
            pb.set_position(0);
            for video in &videos {
                // Convert Video
                if !cli_parse.no_transcode {
                    convert_video(video, &cli_parse);
                }
                pb.inc(1);
            }
            pb.finish_with_message("Encoding done for ${i.clone()}");
        }
        
    }
}
