# Ahref
## Extract "a" tags from html page
### Installation

You can install Ahref using cargo:

```
cargo add ahref
```

### Usage

Here's an example of how to use Ahref:

```rust
use ahref::get_a_tags;

fn main() {
	let html = "<a href='https://github.com/tenqz'>Test link</a>".to_string();
	println!("{:?}", get_a_tags(&html))
}

```

As a result, all "a" tags will be displayed.
