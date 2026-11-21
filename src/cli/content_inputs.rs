use crate::{ContentQuery, ContentRecord, ContentScanOptions, ScanStats};
use std::ffi::OsString;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Read};
use std::ops::ControlFlow;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const MAX_PATH_BYTES: u64 = 1024 * 1024;
const MAX_INPUT_PATHS: usize = 100_000;
const MAX_LIST_BYTES: usize = 64 * 1024 * 1024;

pub(crate) fn path_list(path: &Path, nul: bool) -> io::Result<Vec<PathBuf>> {
    if path == Path::new("-") {
        read_paths(&mut io::stdin().lock(), nul)
    } else {
        read_paths(&mut BufReader::new(File::open(path)?), nul)
    }
}

fn read_paths(reader: &mut impl BufRead, nul: bool) -> io::Result<Vec<PathBuf>> {
    let mut paths = Vec::new();
    let mut input = Vec::new();
    let delimiter = if nul { 0 } else { b'\n' };
    let mut total = 0;
    loop {
        input.clear();
        let read = reader
            .take(MAX_PATH_BYTES + 1)
            .read_until(delimiter, &mut input)?;
        if read == 0 {
            break;
        }
        total += read;
        if read as u64 > MAX_PATH_BYTES || paths.len() == MAX_INPUT_PATHS || total > MAX_LIST_BYTES
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "input path list exceeds its size or entry limit",
            ));
        }
        if input.last() == Some(&delimiter) {
            input.pop();
            if !nul && input.last() == Some(&b'\r') {
                input.pop();
            }
        } else if nul {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "last path is missing a NUL delimiter",
            ));
        }
        if input.is_empty() {
            continue;
        }
        if input.contains(&0) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "input path contains NUL",
            ));
        }
        paths.push(native_path(&input)?);
    }
    Ok(paths)
}

fn native_path(bytes: &[u8]) -> io::Result<PathBuf> {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        Ok(std::ffi::OsStr::from_bytes(bytes).into())
    }
    #[cfg(not(unix))]
    {
        let text = std::str::from_utf8(bytes)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        Ok(text.into())
    }
}

fn decompressor(path: &Path) -> Option<&'static str> {
    let extension = path.extension()?.to_str()?;
    if extension.eq_ignore_ascii_case("gz") {
        Some("gzip")
    } else if extension.eq_ignore_ascii_case("bz2") {
        Some("bzip2")
    } else if extension.eq_ignore_ascii_case("xz") {
        Some("xz")
    } else if extension.eq_ignore_ascii_case("zst") || extension.eq_ignore_ascii_case("zstd") {
        Some("zstd")
    } else {
        None
    }
}

pub(crate) fn scan(
    query: &ContentQuery,
    path: &Path,
    scan_options: &ContentScanOptions,
    stop_after: Option<u64>,
    preprocess: Option<&OsString>,
    decompress: bool,
    mut on_record: impl FnMut(ContentRecord<'_>) -> io::Result<ControlFlow<()>>,
) -> io::Result<ScanStats> {
    let mut command = if let Some(program) = preprocess {
        let mut command = Command::new(program);
        command.arg(path);
        command
    } else if let Some(program) = decompress.then(|| decompressor(path)).flatten() {
        let mut command = Command::new(program);
        command.arg("-dc").arg("--").arg(path);
        command
    } else {
        return query.scan_path(path, scan_options, stop_after, on_record);
    };
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .spawn()?;
    let reader = child.stdout.take().expect("piped child stdout");
    let mut early = false;
    let result = query.scan_reader(reader, scan_options, stop_after, |record| {
        let outcome = on_record(record)?;
        early |= outcome.is_break();
        Ok(outcome)
    });
    early |= match &result {
        Err(_) => true,
        Ok(stats) => {
            stats.binary_skipped || stop_after.is_some_and(|limit| stats.matched_lines >= limit)
        }
    };
    if early {
        let _ = child.kill();
    }
    let status = child.wait()?;
    let stats = result?;
    if !early && !status.success() {
        return Err(io::Error::other(format!(
            "{} exited with {status}",
            command.get_program().to_string_lossy()
        )));
    }
    Ok(stats)
}
