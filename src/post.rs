#[derive(Debug, Deserialize)]
pub struct Post {
    pub ext: Option<String>,
    #[serde(rename = "tim")]
    pub img_id: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct Resp {
    pub posts: Vec<Post>,
}
