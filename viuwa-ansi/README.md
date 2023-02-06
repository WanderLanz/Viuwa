# viuwa-ansi

ANSI library for viuwa

- `AnsiImage` for converting images to ANSI escape sequences
- `Terminal` trait for interacting with the terminal.
- ANSI escape sequence constants in the `consts` module
- ANSI foreground and background colors and escape sequences

## Features

- `image` - Enables some bare minimum support for `image` crate pixels
- `clap` - Derives `ValueEnum` for some Color types
- `rayon` - Enables some API to work with `rayon` crate
- `crossterm` - Replaces some troublesome/non-universal ANSI commands with `crossterm` crate implementations

## Reference

(`{x}` represent a variable)

- ESC = escape = `"\x1B"`
- ST = string terminator = `"\x1B\\"`
- CSI = control sequence introducer = `"\x1B["`
- OSC = operating system command = `"\x1B]"`
- DCS = device control string = `"\x1BP"`
- APM = application program mode = `"\x1B_"`
- SGR = select graphic rendition = `"\x1B[{x}m"`

- vt100:
  - <https://vt100.net/docs/vt100-ug/contents.html>
- fnky's gist:
  - <https://gist.github.com/fnky/458719343aabd01cfb17a3a4f7296797>
- xterm:
  - <https://www.xfree86.org/current/ctlseqs.html>
  - <https://invisible-island.net/xterm/ctlseqs/ctlseqs.html>
- windows:
  - <https://learn.microsoft.com/en-us/windows/console/console-virtual-terminal-sequences>
- linux:
  - <https://en.wikipedia.org/wiki/ANSI_escape_code>
- iterm:
  - <https://iterm2.com/documentation-escape-codes.html>
  - <https://chromium.googlesource.com/apps/libapps/+/master/hterm/doc/ControlSequences.md#OSC-1337>
- kitty:
  - <https://sw.kovidgoyal.net/kitty/graphics-protocol.html>
- alacritty:
  - <https://github.com/alacritty/alacritty/blob/master/docs/escape_support.md>
- mintty:
  - <https://github.com/mintty/mintty/wiki/CtrlSeqs>
- sixel:
  - <https://en.wikipedia.org/wiki/Sixel>
  - <https://konfou.xyz/posts/sixel-for-terminal-graphics>
- sixel spec:
  - <https://vt100.net/docs/vt510-rm/sixel.html>
- 256 colors:
  - <https://robotmoon.com/256-colors>

## Contributing

You are free and welcome to contribute to this project. Please read [CONTRIBUTING.md](../CONTRIBUTING.md) for more information.
