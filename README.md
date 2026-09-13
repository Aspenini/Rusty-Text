# rusty-text

A tiny Rust CLI that writes a phrase to a file a chosen number of times.

Huge counts are streamed in chunks, so memory use stays bounded.

## Features

- Interactive mode (prompts for phrase and repeat count)
- Command mode (`rusty-text "hello" 1000`)
- Optional output path (`-o` / `--output`)
- Buffered streaming writes for very large counts

## Requirements

- Rust 1.85+ (edition 2024)

## Run

```bash
cargo run
```

Then follow prompts.

Or run with arguments:

```bash
cargo run -- "hello world" 1000000
cargo run -- -o out.txt "hello world" 1000000
```

## Build

```bash
cargo build --release
```

## Usage

```
rusty-text [OPTIONS] [PHRASE] [COUNT]

Arguments:
  PHRASE    Text to repeat (prompted if omitted)
  COUNT     Times to repeat; must be > 0 (prompted if omitted)

Options:
  -o, --output <FILE>  Output path [default: output.txt]
  -h, --help           Print help
  -V, --version        Print version
```

`COUNT` may include underscores (`1_000_000`).

## Output

The program writes one repetition per line to:

- `output.txt` (in the current directory), or the path given with `--output`
