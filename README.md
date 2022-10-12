# Viuwa

Viuwa is a simple terminal ANSI image viewer trying to maintain bare-minimum compatibility with the wasm32-wasi target.

It uses almost *exclusively* ANSI escape codes to display
images in the terminal.

Kitty, Sixel, and Iterm2 protocols are not supported,
There are better tools such as [viu](https://github.com/atanunq/viu) or [timg](https://github.com/hzeller/timg) for cross-protocol terminal image viewing, please go and give them love.

Pull requests implementing different protocols are allowed as long as they don't break wasm32-wasi+ANSI compatibility.

## Installation

```bash
cargo install viuwa
```

or for latest version

```bash
git clone https://github.com/WanderLanz/Viuwa.git && cd Viuwa && cargo install --path .
```

## Usage

### Windowed image viewing (e.g. Vim)

```bash
viuwa [image]
```

### Directly to stdout (e.g. Catimg)

```bash
viuwa [image] --inline
```

### For more advanced usage, see the help page

```bash
viuwa --help
```

## Configuration

### Environment variables

- `VIUWA_INLINE`: If set to `true`, viuwa will print the image directly to stdout instead of using the tui.
- `VIUWA_SIZE`: When inline is set, this variable will be used to set the size of the output ANSI image. e.g. `VIUWA_SIZE=100x100` will set the output image to 100x100 characters. (if you have `$COLUMNS` or `$LINES` set, you can also set `VIUWA_SIZE="${COLUMNS}x${LINES}"`)
- `VIUWA_FILTER`: Set the filter type to use when resizing the image. Possible values are `Nearest`, `Triangle`, `CatmullRom`, `Gaussian`, `Lanczos3`. Defaults to `Nearest`.
- `VIUWA_COLOR`: Set the color type of the output ansi image. Possible values are `Truecolor`, `256`, `Grey`. Defaults to `Truecolor`.
- `VIUWA_QUIET`: If set to `true`, viuwa will not print any messages or warnings.

### Configuration file

Not yet implemented, but will be implemented in the future if environment becomes too cumbersome.

## License

This project is licensed under
[MIT](LICENSE-MIT.txt) or [Apache-2.0](LICENSE-APACHE.txt).

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).

## Dependencies

- [clap](https://crates.io/crates/clap)
- [image](https://crates.io/crates/image)
- [crossterm](https://crates.io/crates/crossterm) (on Unix & Windows)
- [ansi_colours](https://crates.io/crates/ansi_colours)
