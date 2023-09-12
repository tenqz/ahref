mod parser;
pub use crate::parser::parser::Parser;

/// Extract html 'a' tag from page
/// # Examples
/// ```
///     use ahref::get_a_tags;
///
///     let html = "<p>Test</p><a href='https://github.com/tenqz/'>Test Link 1</a><p>Another Text</p><a href='https://github.com/tenqz/'>Test Link 2</a><p>Another Text</p><a class='test' href='https://github.com/tenqz/'>Test Link 3</a><p>Another Text</p>".to_string();
///     let tags = get_a_tags(html);
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
pub fn get_a_tags(html: String) -> Vec<String> {
    let mut parser = Parser::new(html);
    parser.parse_tags()
}

/// Extract links from array 'a' tags
/// # Examples
/// ```
///     use ahref::get_url_from_tags;
///     let html = "<p>Test</p><a href='https://github.com/tenqz/'>Test Link 1</a><p>Another Text</p><a href='https://github.com/tenqz/'>Test Link 2</a><p>Another Text</p><a class='test' href='https://github.com/tenqz/'>Test Link 3</a><p>Another Text</p><a href=\"/\" class=\"nav-link px-2 text-muted\">Main Page</a><a href=\"/blog\" class=\"nav-link px-2 text-muted\">Blog</a><a href=\"/vacancies/developer\" class=\"nav-link px-2 text-muted\">Vacancies</a><a href=\'/policy\' class='nav-link px-2 text-muted'>Policy</a>".to_string();
///     let links = get_url_from_tags(html);
///     assert_eq!(
///         vec![
///             "https://github.com/tenqz/",
///             "https://github.com/tenqz/",
///             "https://github.com/tenqz/",
///             "/",
///             "/blog",
///             "/vacancies/developer",
///             "/policy"
///             ],
///         links
///     );
/// ```

pub fn get_url_from_tags(html: String) -> Vec<String> {
    let mut parser = Parser::new(html);
    parser.parse_links()
}
