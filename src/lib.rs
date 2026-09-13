//! Stream a phrase to a writer a given number of times.
//!
//! Each repetition is the phrase followed by a newline. Writes are chunked so
//! huge counts stay O(chunk size) in memory.

use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::path::Path;

/// Buffer size for the file writer (2 MiB).
const WRITER_CAPACITY: usize = 2 * 1024 * 1024;
/// Target size of each pre-filled write (512 KiB).
const CHUNK_BYTES: usize = 512 * 1024;

/// Default output path used by the CLI when none is given.
pub const DEFAULT_OUTPUT: &str = "output.txt";

/// Writes `phrase` followed by a newline, `count` times, to `writer`.
///
/// Memory use is bounded by a fixed-size chunk regardless of `count`. The
/// writer is flushed before this function returns.
pub fn write_repeated<W: Write>(mut writer: W, phrase: &str, count: u64) -> io::Result<()> {
    if count == 0 {
        return writer.flush();
    }

    let line = format!("{phrase}\n");
    let line_bytes = line.as_bytes();
    let line_len = line_bytes.len();

    let reps_per_chunk = (CHUNK_BYTES / line_len).max(1);
    let chunk = line_bytes.repeat(reps_per_chunk);
    let reps_per_chunk = reps_per_chunk as u64;

    let full_chunks = count / reps_per_chunk;
    let leftover = count % reps_per_chunk;

    for _ in 0..full_chunks {
        writer.write_all(&chunk)?;
    }
    if leftover > 0 {
        let end = leftover as usize * line_len;
        debug_assert!(end <= chunk.len());
        writer.write_all(&chunk[..end])?;
    }

    writer.flush()
}

/// Creates (or truncates) `path` and writes `phrase` repeated `count` times.
pub fn write_repeated_to_path(path: impl AsRef<Path>, phrase: &str, count: u64) -> io::Result<()> {
    let path = path.as_ref();
    let file = File::create(path)?;
    let writer = BufWriter::with_capacity(WRITER_CAPACITY, file);
    write_repeated(writer, phrase, count)
}

/// Parses a repeat count: a non-zero `u64`. Underscores are allowed (`1_000`).
pub fn parse_count(s: &str) -> Result<u64, ParseCountError> {
    let s = s.trim();
    if s.is_empty() {
        return Err(ParseCountError::Invalid);
    }

    let normalized = s.replace('_', "");
    match normalized.parse::<u64>() {
        Ok(0) => Err(ParseCountError::Zero),
        Ok(n) => Ok(n),
        Err(_) => Err(ParseCountError::Invalid),
    }
}

/// Failure when parsing a repeat count from text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseCountError {
    /// The text was not an unsigned integer.
    Invalid,
    /// Zero is not a valid repeat count.
    Zero,
}

impl std::fmt::Display for ParseCountError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Invalid => f.write_str("count must be a positive integer"),
            Self::Zero => f.write_str("count must be greater than 0"),
        }
    }
}

impl std::error::Error for ParseCountError {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn writes_phrase_once_per_count() {
        let mut buf = Vec::new();
        write_repeated(&mut buf, "hi", 3).unwrap();
        assert_eq!(buf, b"hi\nhi\nhi\n");
    }

    #[test]
    fn empty_phrase_writes_newlines() {
        let mut buf = Vec::new();
        write_repeated(&mut buf, "", 3).unwrap();
        assert_eq!(buf, b"\n\n\n");
    }

    #[test]
    fn zero_count_writes_nothing() {
        let mut buf = Vec::new();
        write_repeated(&mut buf, "nope", 0).unwrap();
        assert!(buf.is_empty());
    }

    #[test]
    fn preserves_trailing_spaces_in_phrase() {
        let mut buf = Vec::new();
        write_repeated(&mut buf, "ok ", 2).unwrap();
        assert_eq!(buf, b"ok \nok \n");
    }

    #[test]
    fn large_count_has_expected_length() {
        const N: u64 = 10_000;
        let mut buf = Vec::new();
        write_repeated(&mut buf, "ab", N).unwrap();
        assert_eq!(buf.len(), N as usize * 3);
        assert!(buf.starts_with(b"ab\n"));
        assert!(buf.ends_with(b"ab\n"));
    }

    #[test]
    fn phrase_longer_than_chunk_still_repeats() {
        let phrase = "x".repeat(CHUNK_BYTES + 17);
        let mut buf = Vec::new();
        write_repeated(&mut buf, &phrase, 2).unwrap();

        let mut expected = String::new();
        expected.push_str(&phrase);
        expected.push('\n');
        expected.push_str(&phrase);
        expected.push('\n');
        assert_eq!(buf, expected.as_bytes());
    }

    #[test]
    fn writes_to_path() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("rusty-text-{nanos}.txt"));
        write_repeated_to_path(&path, "x", 3).unwrap();
        let contents = fs::read_to_string(&path).unwrap();
        let _ = fs::remove_file(&path);
        assert_eq!(contents, "x\nx\nx\n");
    }

    #[test]
    fn parse_count_accepts_underscores() {
        assert_eq!(parse_count("1_000_000"), Ok(1_000_000));
        assert_eq!(parse_count(" 42 "), Ok(42));
    }

    #[test]
    fn parse_count_rejects_zero_and_junk() {
        assert_eq!(parse_count("0"), Err(ParseCountError::Zero));
        assert_eq!(parse_count(""), Err(ParseCountError::Invalid));
        assert_eq!(parse_count("nope"), Err(ParseCountError::Invalid));
        assert_eq!(parse_count("-1"), Err(ParseCountError::Invalid));
    }
}
