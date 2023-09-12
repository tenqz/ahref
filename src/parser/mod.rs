pub mod parser {
    use lazy_static::lazy_static;
    use regex::Regex;

    pub struct Parser {
    }

    impl Parser {
        pub fn get_a_tags(html: &String) -> Vec<String> {
            lazy_static! {
        static ref HTML_TAG_A_REGEX: Regex = Regex::new(r"<a.*?>.*?<\/a.*?>").unwrap();
    }

            HTML_TAG_A_REGEX
                .find_iter(&html)
                .map(|x| x.as_str().to_string())
                .collect()
        }

        pub fn get_url_from_tags(tags: Vec<String>) -> Vec<String> {
            let mut links = Vec::new();

            for tag in tags {
                let url = Parser::convert_a_href_to_url(tag);
                links.push(url.to_string());
            }

            links
        }

        fn convert_a_href_to_url(tag_a: String)-> String {
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
}

