# Viuwa
[![crate](https://img.shields.io/crates/v/viuwa.svg?style=for-the-badge)](https://crates.io/crates/viuwa) <!-- [![crate all releases](https://img.shields.io/crates/d/viuwa?color=fe7d37&style=for-the-badge)](https://crates.io/crates/viuwa) -->
[![github](https://img.shields.io/github/v/release/WanderLanz/Viuwa?include_prereleases&label=GITHUB&style=for-the-badge)](https://github.com/WanderLanz/Viuwa) <!-- [![github all releases](https://img.shields.io/github/downloads/WanderLanz/Viuwa/total?style=for-the-badge)](https://github.com/WanderLanz/Viuwa/releases) -->
[![license](https://img.shields.io/crates/l/viuwa.svg?style=for-the-badge)](NOTICES.txt)

Viuwa is a simple terminal ANSI image viewer trying to maintain bare-minimum compatibility with the wasm32-wasi target.

It uses almost *exclusively* ANSI escape codes to display
images in the terminal.

Kitty, Sixel, and Iterm2 protocols are not supported,
There are better tools such as [viu](https://github.com/atanunq/viu) or [timg](https://github.com/hzeller/timg) for cross-protocol terminal image viewing, please go and give them love.

Pull requests implementing different protocols are allowed as long as they don't break wasm32-wasi+ANSI compatibility.

## âšī¸ Installation

```bash
cargo install viuwa
```

or for latest version

```bash
git clone https://github.com/WanderLanz/Viuwa.git && cd Viuwa && cargo install --path .
```

`wasm` file is also available in the releases section (with `rayon` feature disabled).

### Features

  - `rayon`: Enables both parallel image resizing and conversions to ANSI. This is enabled by default.

## đ§ Usage

### Windowed image viewing (e.g. Vim)

```bash
viuwa [image]
```

### Directly to command line (e.g. Catimg)

```bash
viuwa [image] --inline
```

### For more advanced usage, see the help

```bash
viuwa --help
```

## âī¸ Configuration

### Environment variables

- `VIUWA_QUIET`: If set to `true`, viuwa will not print any messages or warnings.
- `VIUWA_FILTER`: Set the filter type to use when resizing the image. Possible values are `Nearest`, `Triangle`, `CatmullRom`, `Gaussian`, `Lanczos3`. Defaults to `Nearest`.
- `VIUWA_COLOR`: Set the color type of the output ansi image. Possible values are `Truecolor`, `256`, `Gray`, and `256Gray`. Defaults to `Truecolor`.
- `VIUWA_CORRECT`: Set the luma correction level for 256 color mode, allows more pixels to be converted to grayscale for better contrast. 0-100, Defaults to `100`.

With inline flag:

- `VIUWA_INLINE`: If set to `true`, viuwa will inline the resulting ANSI image instead of using a tui.
- `VIUWA_WIDTH`: Set width of inlined ANSI image, else does nothing.
- `VIUWA_HEIGHT`: Set height of inlined ANSI image, else does nothing.

### đ¸ Examples

inlined w/ nearest filter

![cli-f1](/img/lights-inline.png)

tui w/ triangle filter

![tui-f2](/img/lights-tui-triangle.png)

tui help

![tui-help](/img/viuwa-tui-help.png)

## đŦ Known Issues

- On wasm, ANSI raw mode sequences are commonly ignored, so you may need to press enter to send input to the program.
- Some wasm runtimes may kill the program waiting for user input. May cause terminal to be left in a weird state.

## đ License

This project is licensed under
[MIT](LICENSE-MIT.txt) or [Apache-2.0](LICENSE-APACHE.txt).

## đĄ Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).

## <a href="https://crates.io"><image alt="crates.io" src="/img/crates-io-favicon.ico.svg"/></a> Dependencies

[![clap crate](https://img.shields.io/static/v1?label=đĻ&message=clap&color=B94700&style=flat-square)](https://crates.io/crates/clap)
[![image crate](https://img.shields.io/static/v1?label=đĻ&message=image&color=B94700&style=flat-square)](https://crates.io/crates/image)
[![rayon crate](https://img.shields.io/static/v1?label=đĻ&message=rayon&color=B94700&style=flat-square)](https://crates.io/crates/rayon)
[![ndarray crate](https://img.shields.io/static/v1?label=đĻ&message=ndarray&color=B94700&style=flat-square)](https://crates.io/crates/ndarray)
[![crossterm crate](https://img.shields.io/static/v1?label=đĻ&message=crossterm&color=B94700&style=flat-square)](https://crates.io/crates/crossterm)
