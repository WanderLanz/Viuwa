# Viuwa

Viuwa is a simple terminal ANSI image viewer trying to maintain bare-minimum compatibility with the wasm-wasi target.

We use the [image](https://crates.io/crates/image), [clap](https://crates.io/crates/clap), and the [crossterm](https://crates.io/crates/crossterm) crates
for cli tooling, image manipulation, and UNIX/Windows platform specific tui tooling.

It uses exlusively ANSI escape codes to display
images in the terminal.

Kitty, Sixel, and Iterm2 protocols are not supported,
There are better tools such as [viu](https://github.com/atanunq/viu) or [timg](https://github.com/hzeller/timg) for cross-protocol terminal image viewing.

Pull requests implementing different protocols are allowed as long as they don't break Wasi+ANSI compatibility.

## Usage

```pwsh
viuwa [image]
```

## Installation

```pwsh
cargo install viuwa
```

## License

This project is licensed under
[MIT](LICENSE-MIT.txt) or [Apache-2.0](LICENSE-APACHE.txt).

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).

## Dependencies

- [clap](https://crates.io/crates/clap)
- [image](https://crates.io/crates/image)
- [crossterm](https://crates.io/crates/crossterm)
