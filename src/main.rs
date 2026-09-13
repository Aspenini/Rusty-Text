//! Repeat a phrase to a file N times. Huge N is streamed, not buffered whole.
//!
//! Usage:
//!   rusty-text                         # prompt for phrase and count
//!   rusty-text "phrase" 1000000        # write "phrase" 1M times
//!   rusty-text -o out.txt "hi" 10      # choose the output path

use std::io::{self, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use rusty_text::{DEFAULT_OUTPUT, parse_count, write_repeated_to_path};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("error: {err}");
            if matches!(err, Error::Usage(_)) {
                eprintln!(
                    "Try `{bin} --help` for more information.",
                    bin = env!("CARGO_BIN_NAME")
                );
                ExitCode::from(2)
            } else {
                ExitCode::FAILURE
            }
        }
    }
}

fn run() -> Result<(), Error> {
    match parse_args(std::env::args().skip(1))? {
        Command::Help => {
            print_help();
            Ok(())
        }
        Command::Version => {
            println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        Command::Run {
            phrase,
            count,
            output,
        } => {
            let phrase = match phrase {
                Some(p) => p,
                None => read_phrase()?,
            };
            let count = match count {
                Some(n) => n,
                None => read_count()?,
            };
            write_repeated_to_path(&output, &phrase, count).map_err(|source| Error::Io {
                path: output.clone(),
                source,
            })?;
            println!(
                "Done. Wrote {count} repetitions to {path}",
                path = output.display()
            );
            Ok(())
        }
    }
}

#[derive(Debug)]
enum Command {
    Help,
    Version,
    Run {
        phrase: Option<String>,
        count: Option<u64>,
        output: PathBuf,
    },
}

#[derive(Debug)]
enum Error {
    Io { path: PathBuf, source: io::Error },
    Usage(String),
    InvalidCount(String),
    Input(io::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io { path, source } => {
                write!(f, "failed to write {}: {source}", path.display())
            }
            Self::Usage(msg) | Self::InvalidCount(msg) => f.write_str(msg),
            Self::Input(source) => write!(f, "failed to read input: {source}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } | Self::Input(source) => Some(source),
            Self::Usage(_) | Self::InvalidCount(_) => None,
        }
    }
}

fn parse_args<I, S>(args: I) -> Result<Command, Error>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut output = PathBuf::from(DEFAULT_OUTPUT);
    let mut positional = Vec::new();
    let mut options_ended = false;
    let mut iter = args.into_iter();

    while let Some(arg) = iter.next() {
        let arg = arg.as_ref();
        if !options_ended {
            match arg {
                "-h" | "--help" => return Ok(Command::Help),
                "-V" | "--version" => return Ok(Command::Version),
                "-o" | "--output" => {
                    let Some(path) = iter.next() else {
                        return Err(Error::Usage("--output requires a path".to_string()));
                    };
                    output = PathBuf::from(path.as_ref());
                    continue;
                }
                "--" => {
                    options_ended = true;
                    continue;
                }
                other => {
                    if let Some(path) = other.strip_prefix("--output=") {
                        output = PathBuf::from(path);
                        continue;
                    }
                    if other.starts_with('-') && other != "-" {
                        return Err(Error::Usage(format!("unknown option: {other}")));
                    }
                }
            }
        }
        positional.push(arg.to_string());
    }

    match positional.len() {
        0 => Ok(Command::Run {
            phrase: None,
            count: None,
            output,
        }),
        1 => Err(Error::Usage(
            "missing count; expected PHRASE and COUNT".to_string(),
        )),
        2 => {
            let count = parse_count(&positional[1])
                .map_err(|e| Error::InvalidCount(format!("{e}: {}", positional[1])))?;
            let phrase = positional.into_iter().next();
            Ok(Command::Run {
                phrase,
                count: Some(count),
                output,
            })
        }
        _ => Err(Error::Usage("too many arguments".to_string())),
    }
}

fn print_help() {
    let bin = env!("CARGO_BIN_NAME");
    let version = env!("CARGO_PKG_VERSION");
    println!(
        "\
{bin} {version}

Repeat a phrase to a file. Huge counts are streamed in chunks.

Usage:
  {bin} [OPTIONS] [PHRASE] [COUNT]

Arguments:
  PHRASE    Text to repeat (prompted if omitted)
  COUNT     Times to repeat; must be > 0 (prompted if omitted)

Options:
  -o, --output <FILE>  Output path [default: {DEFAULT_OUTPUT}]
  -h, --help           Print help
  -V, --version        Print version"
    );
}

fn read_phrase() -> Result<String, Error> {
    prompt_line("Enter the phrase: ")
}

fn read_count() -> Result<u64, Error> {
    loop {
        let s = prompt_line("How many times should it repeat? ")?;
        match parse_count(&s) {
            Ok(n) => return Ok(n),
            Err(e) => println!("{e}."),
        }
    }
}

fn prompt_line(prompt: &str) -> Result<String, Error> {
    print!("{prompt}");
    io::stdout().flush().map_err(Error::Input)?;

    let mut line = String::new();
    let n = io::stdin().read_line(&mut line).map_err(Error::Input)?;
    if n == 0 {
        return Err(Error::Input(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "unexpected end of input",
        )));
    }

    let end = line.trim_end_matches(['\r', '\n']).len();
    line.truncate(end);
    Ok(line)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse<const N: usize>(args: [&str; N]) -> Command {
        parse_args(args).expect("args should parse")
    }

    #[test]
    fn no_args_is_interactive() {
        let Command::Run {
            phrase,
            count,
            output,
        } = parse([])
        else {
            panic!("expected Run");
        };
        assert!(phrase.is_none());
        assert!(count.is_none());
        assert_eq!(output, PathBuf::from(DEFAULT_OUTPUT));
    }

    #[test]
    fn phrase_and_count() {
        let Command::Run {
            phrase,
            count,
            output,
        } = parse(["hello", "1000"])
        else {
            panic!("expected Run");
        };
        assert_eq!(phrase.as_deref(), Some("hello"));
        assert_eq!(count, Some(1000));
        assert_eq!(output, PathBuf::from(DEFAULT_OUTPUT));
    }

    #[test]
    fn output_flag_and_equals_form() {
        let Command::Run { output, .. } = parse(["-o", "out.txt", "x", "1"]) else {
            panic!("expected Run");
        };
        assert_eq!(output, PathBuf::from("out.txt"));

        let Command::Run { output, .. } = parse(["--output=dest.txt", "x", "1"]) else {
            panic!("expected Run");
        };
        assert_eq!(output, PathBuf::from("dest.txt"));
    }

    #[test]
    fn help_and_version() {
        assert!(matches!(parse(["--help"]), Command::Help));
        assert!(matches!(parse(["-V"]), Command::Version));
    }

    #[test]
    fn double_dash_allows_phrase_that_looks_like_a_flag() {
        let Command::Run { phrase, count, .. } = parse(["--", "--help", "3"]) else {
            panic!("expected Run");
        };
        assert_eq!(phrase.as_deref(), Some("--help"));
        assert_eq!(count, Some(3));
    }

    #[test]
    fn missing_count_is_usage_error() {
        assert!(matches!(parse_args(["only-phrase"]), Err(Error::Usage(_))));
    }

    #[test]
    fn unknown_option_is_usage_error() {
        assert!(matches!(parse_args(["--nope"]), Err(Error::Usage(_))));
    }

    #[test]
    fn invalid_count_is_error() {
        assert!(matches!(
            parse_args(["hi", "0"]),
            Err(Error::InvalidCount(_))
        ));
        assert!(matches!(
            parse_args(["hi", "abc"]),
            Err(Error::InvalidCount(_))
        ));
    }
}
