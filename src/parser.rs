use lazy_static::lazy_static;
use regex::Regex;

pub struct Parser {
    html: String,
}

impl Parser {
    pub fn new(html: String) -> Self {
        Parser { html }
    }

    pub fn parse_tags(&mut self) -> Vec<String> {
        lazy_static! {
                static ref HTML_TAG_A_REGEX: Regex = Regex::new(r"<a.*?>.*?<\/a.*?>").unwrap();
            }

        HTML_TAG_A_REGEX
            .find_iter(&self.html)
            .map(|x| x.as_str().to_string())
            .collect()
    }

    pub fn parse_links(&mut self) -> Vec<String> {
        let mut links = Vec::new();

        for tag in self.parse_tags() {
            let url = self.convert_a_href_to_url(tag);
            links.push(url.to_string());
        }

        links
    }

    fn convert_a_href_to_url(&mut self, tag_a: String) -> String {
        let regex_string = "href='.+'|href=\".+\"";
        let regex_link = Regex::new(regex_string).unwrap();

        let mut url = regex_link
            .find(&tag_a)
            .unwrap()
            .as_str()
            .replace("href=", "")
            .replace("\'", "")
            .replace("\"", "");

        // remove after space anything that is not a hyperlink
        let offset = url.find(" ").unwrap_or(url.len());
        url.replace_range(offset.., "");

        url
    }
}
