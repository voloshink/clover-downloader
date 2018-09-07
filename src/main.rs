extern crate clap;
extern crate regex;
extern crate reqwest;
extern crate serde_json;

use std::fs;
use std::io::copy;
use std::path::PathBuf;
use std::sync::mpsc;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;

#[macro_use]
extern crate serde_derive;

use clap::{App, Arg};
mod ch_thread;

const THREAD_COUNT: usize = 5;

fn main() {
    let matches = App::new("Clover Downloader")
        .version("1.0")
        .about("Downloads images/gifs from a given 4chan thread")
        .after_help("Either Provide a Thread URL or a Board and ID")
        .arg(
            Arg::with_name("board")
                .short("b")
                .long("board")
                .value_name("BOARD")
                .help("The Board Hosting the Thread"),
        ).arg(
            Arg::with_name("id")
                .short("i")
                .long("id")
                .value_name("ID")
                .help("The ID of the Thread"),
        ).arg(
            Arg::with_name("url")
                .short("u")
                .long("url")
                .value_name("URL")
                .help("The URL of the thread"),
        ).arg(
            Arg::with_name("directory")
                .short("d")
                .long("directory")
                .value_name("PATH")
                .required(true)
                .help("The Directory Where the Files will be Saved"),
        ).arg(
            Arg::with_name("include_animated")
                .short("a")
                .long("animated")
                .help("Include Animated Files (gif, webm etc.)"),
        ).get_matches();

    let url = matches.value_of("url");
    let board = matches.value_of("board");
    let id = matches.value_of("id");
    let directory = PathBuf::from(matches.value_of("directory").unwrap());

    fs::create_dir_all(&directory).expect("Error creating directory");

    let thread = match (url, board, id) {
        (Some(url), _, _) => ch_thread::Thread::from_url(url),
        (None, Some(board), Some(id)) => ch_thread::Thread::from_id(board, id),
        _ => panic!("Please either provide a board and an id or a url to the thread"),
    };

    let response = thread.get_json();

    let mut formats: Vec<&str> = Vec::new();
    formats.append(&mut vec![".jpg", ".png", ".jpeg"]);
    if matches.is_present("include_animated") {
        formats.append(&mut vec![".webm", ".gif"]);
    }

    let file_posts: Vec<ch_thread::Post> = response
        .posts
        .into_iter()
        .filter(|p| formats.contains(&&p.ext[..]))
        .collect();

    let file_urls: Vec<String> = file_posts
        .into_iter()
        .map(|x| get_file_url(thread.get_board(), x))
        .flat_map(|e| e)
        .collect();

    download_files(directory, file_urls);
}

fn get_file_url(board: &str, post: ch_thread::Post) -> Option<String> {
    if post.tim != 0 && !post.ext.is_empty() {
        Some(format!(
            "https://i.4cdn.org/{}/{}{}",
            board, post.tim, post.ext
        ))
    } else {
        None
    }
}

enum Message {
    File(String),
    Terminate,
}

fn download_files(directory: PathBuf, urls: Vec<String>) {
    let (sender, receiver) = mpsc::channel();
    let receiver = Arc::new(Mutex::new(receiver));
    let directory = Arc::new(directory);

    let mut threads: Vec<thread::JoinHandle<()>> = Vec::new();

    for _ in 0..THREAD_COUNT {
        let recv = Arc::clone(&receiver);
        let dir = Arc::clone(&directory);
        let thread = thread::spawn(move || loop {
            let message = recv.lock().unwrap().recv().unwrap();

            match message {
                Message::File(url) => {
                    let result = download_file(&dir, &url);
                }
                Message::Terminate => break,
            }
        });
        threads.push(thread)
    }

    for url in urls {
        sender.send(Message::File(url)).unwrap();
    }

    for _ in &threads {
        sender.send(Message::Terminate).unwrap();
    }

    for thread in threads {
        thread.join().unwrap();
    }
}

fn download_file(directory: &PathBuf, url: &str) -> Result<(), Box<std::error::Error>> {
    let mut response = reqwest::get(url)?;
    let fname = response
        .url()
        .path_segments()
        .and_then(|segments| segments.last())
        .and_then(|name| if name.is_empty() { None } else { Some(name) })
        .unwrap()
        .to_string();
    let fname = directory.join(fname);
    let mut file = fs::File::create(fname)?;
    copy(&mut response, &mut file)?;
    Ok(())
}
