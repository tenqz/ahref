use lazy_static::lazy_static;
use regex::Regex;

pub fn get_a_tags(html: &String) -> Vec<String> {
    lazy_static! {
        static ref HTML_TAG_A_REGEX: Regex = Regex::new(r"<a.*?>.*?<\/a.*?>").unwrap();
    }

    HTML_TAG_A_REGEX
        .find_iter(&html)
        .map(|x| x.as_str().to_string())
        .collect()
}
