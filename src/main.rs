extern crate clap;
extern crate regex;
extern crate reqwest;
extern crate serde_json;

use regex::Regex;
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
mod post;

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
        ).arg(
            Arg::with_name("verbose")
                .short("v")
                .long("verbose")
                .help("Print out urls that were succefully downloaded"),
        ).get_matches();

    let mut url = matches.value_of("url").map(str::to_string);
    let mut board = matches.value_of("board").map(str::to_string);
    let id = matches.value_of("id");
    let directory = PathBuf::from(matches.value_of("directory").unwrap());

    fs::create_dir_all(&directory).expect("Error creating directory");

    match (url, board, id) {
        (Some(link), _, _) => {
            board = Some(board_from_url(&link));
            url = Some(link);
        }
        (None, Some(b), Some(id)) => {
            url = Some(url_from_board_and_id(&b, id));
            board = Some(b);
        }
        _ => panic!("Please either provide a board and an id or a url to the thread"),
    };

    let url = url.unwrap();
    let board = board.unwrap();

    let response = get_thread(&url);

    let mut formats = vec![".jpg", ".png", ".jpeg"];
    if matches.is_present("include_animated") {
        formats.extend_from_slice(&[".webm", ".gif"]);
    }

    let file_posts: Vec<post::Post> = response
        .posts
        .into_iter()
        .filter(|p| formats.contains(&&p.ext[..]))
        .collect();

    let file_urls: Vec<String> = file_posts
        .into_iter()
        .map(|x| get_file_url(&board, x))
        .flat_map(|e| e)
        .collect();

    download_files(directory, file_urls, matches.is_present("verbose"));
}

fn board_from_url(url: &str) -> String {
    let re = Regex::new(
        r"^(https://|http://)?(www\.)?boards\.4chan\.org/(?P<board>\D+)/thread/(?P<id>\d+)$",
    ).unwrap();

    if !re.is_match(url) {
        panic!("Provided URL was in an invalid format, expected \"boards.4chan.org/{board}/thread/{id}\"");
    }

    let captures = re.captures(url).unwrap();

    captures.name("board").unwrap().as_str().to_string()
}

fn url_from_board_and_id(board: &str, id: &str) -> String {
    format!("https://boards.4chan.org/{}/thread/{}", board, id)
}

fn get_thread(url: &str) -> post::Resp {
    use self::reqwest::StatusCode;

    let mut url = url.to_owned();
    url.push_str(".json");
    let mut res = reqwest::get(&url).expect("Error perorming GET request");

    if res.status() != StatusCode::Ok {
        panic!(format!("Received non-ok status code {}", res.status()));
    }

    res.json::<post::Resp>()
        .expect("Error parsing response JSON")
}

fn get_file_url(board: &str, post: post::Post) -> Option<String> {
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

fn download_files(directory: PathBuf, urls: Vec<String>, verbose: bool) {
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
                    match result {
                        Ok(_) => {
                            if verbose {
                                println!("Downloaded {}", url)
                            }
                        }
                        Err(e) => {
                            println!("Error downloading image {}: {}", url, e.description());
                        }
                    }
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
