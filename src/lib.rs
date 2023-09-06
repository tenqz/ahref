use lazy_static::lazy_static;
use regex::Regex;

/// Extract html 'a' tag from page
/// # Examples
/// ```
///     use ahref::get_a_tags;
///
///     let html = "<p>Test</p><a href='https://github.com/tenqz/'>Test Link 1</a><p>Another Text</p><a href='https://github.com/tenqz/'>Test Link 2</a><p>Another Text</p><a class='test' href='https://github.com/tenqz/'>Test Link 3</a><p>Another Text</p>".to_string();
///     let tags = get_a_tags(&html);
///     assert_eq!(
///        vec![
///            "<a href='https://github.com/tenqz/'>Test Link 1</a>".to_string(),
///            "<a href='https://github.com/tenqz/'>Test Link 2</a>".to_string(),
///            "<a class='test' href='https://github.com/tenqz/'>Test Link 3</a>".to_string()
///        ],
///        tags
///    );
///
/// ```
pub fn get_a_tags(html: &String) -> Vec<String> {
    lazy_static! {
        static ref HTML_TAG_A_REGEX: Regex = Regex::new(r"<a.*?>.*?<\/a.*?>").unwrap();
    }

    HTML_TAG_A_REGEX
        .find_iter(&html)
        .map(|x| x.as_str().to_string())
        .collect()
}

/// Extract link from array 'a' tags
/// # Examples
/// ```
///     use ahref::get_url_from_tags;
///     let tags = vec![
///         "<a href='https://github.com/tenqz/'>Test Link 1</a>".to_string(),
///         "<a href='https://github.com/tenqz/'>Test Link 2</a>".to_string(),
///         "<a class='test' href='https://github.com/tenqz/'>Test Link 3</a>".to_string()
///     ];
///     let links = get_url_from_tags(tags);
///     assert_eq!(
///         vec!["https://github.com/tenqz/","https://github.com/tenqz/","https://github.com/tenqz/"],
///         links
///     );
/// ```

pub fn get_url_from_tags(tags: Vec<String>) -> Vec<String> {
    let regex_string = "href='.+'|href=\".+\"";
    let regex_link = Regex::new(regex_string).unwrap();
    let mut links = Vec::new();

    for tag in tags {
        // str = str.trim_matches("\'");
        links.push(
            regex_link
                .find(&tag)
                .unwrap()
                .as_str()
                .replace("href=", "")
                .replace("\'", "")
                .replace("\"", "")
                .to_string(),
        );
    }

    links
}
