use ahref::parser::Parser;
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    let html = &args[1];

    let mut parser = Parser::new(html.to_string());

    println!("{:?}", parser.parse_tags());
}
