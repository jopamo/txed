use crate::cli::ApplyArgs;
use crate::error::{Error, Result};
use std::io::{self, BufRead, Read, BufReader};
use std::path::PathBuf;
use crate::rgjson::{RgMessage, RgKind, stream_rg_json_ndjson, RgSink};

#[derive(Debug, PartialEq, Eq)]
pub enum InputMode {
    /// Read paths from command line arguments.
    /// If no args, and stdin is a pipe, read paths from stdin (newline delimited).
    Auto(Vec<PathBuf>),
    /// Read paths from stdin (newline delimited).
    StdinPathsNewline,
    /// Read paths from stdin (NUL delimited).
    StdinPathsNul,
    /// Read content from stdin.
    StdinText,
    /// Read ripgrep JSON from stdin.
    RipgrepJson,
}

#[derive(Debug)]
pub enum InputItem {
    Path(PathBuf),
    StdinText(String),
    // RgSpan { ... } // Future
}

pub fn resolve_input_mode(args: &ApplyArgs) -> InputMode {
    if args.stdin_text {
        InputMode::StdinText
    } else if args.rg_json {
        InputMode::RipgrepJson
    } else if args.files0 {
        InputMode::StdinPathsNul
    } else if args.stdin_paths {
        InputMode::StdinPathsNewline
    } else {
        InputMode::Auto(args.files.clone())
    }
}

/// Read newline-delimited paths from stdin.
pub fn read_paths_from_stdin() -> Result<Vec<PathBuf>> {
    let stdin = io::stdin();
    let mut paths = Vec::new();
    for line in stdin.lock().lines() {
        let line = line.map_err(Error::Io)?;
        if !line.trim().is_empty() {
            paths.push(PathBuf::from(line.trim()));
        }
    }
    Ok(paths)
}

/// Read NUL-delimited paths from stdin.
pub fn read_paths_from_stdin_zero() -> Result<Vec<PathBuf>> {
    let stdin = io::stdin();
    let mut handle = stdin.lock();
    let mut paths = Vec::new();
    let mut buf = Vec::new();
    
    // read_until includes the delimiter
    while handle.read_until(0, &mut buf).map_err(Error::Io)? > 0 {
        // Remove the trailing NUL
        if let Some(&0) = buf.last() {
            buf.pop();
        }
        if !buf.is_empty() {
             let s = String::from_utf8(buf.clone())
                .map_err(|e| Error::Validation(format!("Invalid UTF-8 in path: {}", e)))?;
             paths.push(PathBuf::from(s));
        }
        buf.clear();
    }
    Ok(paths)
}

/// Read all text from stdin.
pub fn read_stdin_text() -> Result<String> {
    let mut buffer = String::new();
    // Check if stdin is tty? No, if mode is StdinText we assume they want to read from it.
    // But if it is a TTY we might hang.
    // However, logic usually checks atty before calling this if in Auto mode.
    // In StdinText mode, we force read.
    io::stdin().read_to_string(&mut buffer).map_err(Error::Io)?;
    Ok(buffer)
}

struct PathCollectorSink {
    paths: Vec<PathBuf>,
}

impl RgSink for PathCollectorSink {
    fn handle(&mut self, msg: RgMessage) -> anyhow::Result<()> {
        match msg.kind {
            RgKind::Begin => {
                if let Some(data) = msg.data {
                    if let Some(path_obj) = data.path {
                        let os_str = path_obj.to_os_string()?;
                        self.paths.push(PathBuf::from(os_str));
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }
}

/// Read ripgrep JSON output and extract paths.
/// Uses robust handling for bytes vs text in paths.
pub fn read_rg_json() -> Result<Vec<PathBuf>> {
    let stdin = io::stdin();
    let reader = BufReader::new(stdin.lock());
    let mut sink = PathCollectorSink { paths: Vec::new() };
    
    stream_rg_json_ndjson(reader, &mut sink).map_err(|e| Error::Validation(format!("Failed to parse rg json: {}", e)))?;
    
    // Deduplicate? Rg usually groups by file, but we might get multiple blocks?
    // A simple vector is fine for now, dedup can happen later if needed.
    sink.paths.sort();
    sink.paths.dedup();
    Ok(sink.paths)
}
