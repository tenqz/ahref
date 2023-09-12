# Ahref

## Extract "a" tags from html page

### Installation

You can install Ahref using cargo:

```
cargo add ahref
```

### Usage

Here's an example of how to use Ahref lib:

```rust
use ahref::Parser;

fn main() {
    let html = "<a href='https://github.com/tenqz'>Test link</a>".to_string();
    let mut parser = Parser::new(html);
    println!("{:?}", parser.parse_tags());
}

```

As a result, all "a" tags will be displayed.
```text
["<a href='https://github.com/tenqz'>Test link</a>"]
```


```rust
use ahref::Parser;

fn main() {
    let html = "<a href='https://github.com/tenqz'>Test link</a>".to_string();
    let mut parser = Parser::new(html);
    println!("{:?}", parser.parse_links());
}

```
As a result, all urls will be displayed.
```text
["https://github.com/tenqz"]
```
