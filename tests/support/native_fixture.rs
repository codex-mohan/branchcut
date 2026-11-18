/// Create an unusual native filename only when the fixture filesystem permits
/// it. APFS/HFS can reject invalid UTF-8 with EILSEQ before Branchcut runs.
pub fn write(path: impl AsRef<std::path::Path>, contents: &[u8]) -> bool {
    match std::fs::write(path.as_ref(), contents) {
        Ok(()) => true,
        Err(error) if cfg!(target_os = "macos") && error.raw_os_error() == Some(92) => {
            eprintln!(
                "fixture filesystem rejects invalid UTF-8 filenames (EILSEQ); byte-path assertion requires a byte-preserving filesystem"
            );
            false
        }
        Err(error) => panic!("create native filename fixture: {error}"),
    }
}
