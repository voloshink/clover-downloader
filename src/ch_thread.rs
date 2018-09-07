extern crate reqwest;

use regex::Regex;

pub struct Thread<'a> {
    url: String,
    board: &'a str,
    id: &'a str,
}

impl<'a> Thread<'a> {
    pub fn from_id(board: &'a str, id: &'a str) -> Thread<'a> {
        let url = format!("https://boards.4chan.org/{}/thread/{}", board, id);
        Thread {
            url: url,
            board,
            id,
        }
    }

    pub fn from_url(url: &'a str) -> Thread<'a> {
        let re = Regex::new(
            r"^(https://|http://)?(www\.)?boards\.4chan\.org/(?P<board>\D+)/thread/(?P<id>\d+)$",
        ).unwrap();

        if !re.is_match(url) {
            panic!("Provided URL was in an invalid format, expected \"boards.4chan.org/{board}/thread/{id}\"");
        }

        let captures = re.captures(url).unwrap();

        Thread {
            url: String::from(url),
            board: captures.name("board").unwrap().as_str(),
            id: captures.name("id").unwrap().as_str(),
        }
    }

    pub fn url(&self) -> &str {
        &self.url[..]
    }

    pub fn get_board(&self) -> &str {
        self.board
    }

    pub fn get_json(&self) -> Resp {
        use self::reqwest::StatusCode;

        let mut url = self.url.to_owned();
        url.push_str(".json");
        let mut res = reqwest::get(&url).expect("Error perorming GET request");

        if res.status() != StatusCode::Ok {
            panic!(format!("Received non-ok status code {}", res.status()));
        }

        res.json::<Resp>().expect("Error parsing response JSON")
    }
}

#[derive(Debug, Deserialize)]
pub struct Post {
    #[serde(default)]
    pub ext: String,

    #[serde(default)]
    pub tim: u64,
}

#[derive(Debug, Deserialize)]
pub struct Resp {
    pub posts: Vec<Post>,
}
