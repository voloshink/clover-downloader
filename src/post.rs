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
