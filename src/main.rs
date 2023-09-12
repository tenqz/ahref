use ahref::get_a_tags;
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    let html = &args[1];

    println!("{:?}", get_a_tags(html.to_string()));
}
