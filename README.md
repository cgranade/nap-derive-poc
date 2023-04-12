# nap-derive-poc

This repo provides very rough proof of concept for a clap_derive-style macro to help write nushell plugins.

To demonstrate, this repo also includes a small nushell plugin that searches Scryfall for Magic: The Gathering cards. I might add more silly little commands like that at some point.

## Why "nap"?

"clap" expands to "command-line argument parser," so I figured "nap" could expand to "nushell argument parser"? It's funny, even if not fully technically correct (nushell itself does the parsing). Whimsey wins.

## Disclaimer

Did I say this is a proof of concept yet? Because it is. The code is a mess, and not maintainable as-is, but the goal was to demonstrate the concept more than to be usable in its current form.

As such, I won't post this to crates.io in its current state.

## Building

Run the following in nushell:

```nushell
cargo build
# On Linux, macOS, and similar:
register target/debug/nu_plugin_mtg
# On Windows:
register target/debug/nu_plugin_mtg.exe
```
