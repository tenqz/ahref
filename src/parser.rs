use regex::Regex;
use tagparser;

pub struct Parser {
    html: String,
}

impl Parser {
    pub fn new(html: String) -> Self {
        Parser { html }
    }

    pub fn parse_tags(&mut self) -> Vec<String> {
        tagparser::parse_tags(self.html.clone(), "a".to_string())
    }

    pub fn parse_links(&mut self) -> Vec<String> {
        let mut links = Vec::new();

        for tag in self.parse_tags() {
            if self.create_regex().is_match(&tag) {
                let url = self.convert_a_href_to_url(tag);
                links.push(url.to_string());
            }
        }

        links
    }

    fn convert_a_href_to_url(&mut self, tag_a: String) -> String {
        let regex_link = self.create_regex();

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

    fn create_regex(&self) -> Regex {
        let regex_string = "href='.+'|href=\".+\"";

        Regex::new(regex_string).unwrap()
    }
}
