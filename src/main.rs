use lazy_static::lazy_static;
use regex::Regex;
use std::env;

fn get_links(html: &String) -> Vec<String> {
    lazy_static! {
        static ref HTML_TAG_A_REGEX: Regex = Regex::new(r"<a.*?>.*?<\/a.*?>").unwrap();
    }

    HTML_TAG_A_REGEX
        .find_iter(&html)
        .map(|x| x.as_str().to_string())
        .collect()
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let html = &args[1];

    println!("{:?}", get_links(html));
}
