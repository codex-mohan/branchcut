use std::ffi::{OsStr, OsString};
#[cfg(unix)]
use std::os::unix::ffi::OsStrExt;
use std::path::{Component, Path};
#[cfg(unix)]
pub(crate) fn with_os_bytes<T>(value: &OsStr, function: impl FnOnce(&[u8]) -> T) -> T {
    function(value.as_bytes())
}

#[cfg(not(unix))]
pub(crate) fn with_os_bytes<T>(value: &OsStr, function: impl FnOnce(&[u8]) -> T) -> T {
    function(value.as_encoded_bytes())
}
#[cfg(unix)]
pub(crate) fn os_string_from_pattern_bytes(bytes: &[u8]) -> OsString {
    use std::os::unix::ffi::OsStringExt;
    OsString::from_vec(bytes.to_vec())
}

#[cfg(not(unix))]
pub(crate) fn os_string_from_pattern_bytes(bytes: &[u8]) -> OsString {
    OsString::from(String::from_utf8_lossy(bytes).into_owned())
}
pub(crate) fn normal_components(path: &Path) -> Vec<&OsStr> {
    path.components()
        .filter_map(|component| match component {
            Component::Normal(value) => Some(value),
            _ => None,
        })
        .collect()
}
pub(crate) fn is_hidden(name: &OsStr) -> bool {
    with_os_bytes(name, |bytes| {
        bytes.first() == Some(&b'.') && bytes != b"." && bytes != b".."
    })
}

pub(crate) fn path_has_hidden_component(path: &Path) -> bool {
    normal_components(path).into_iter().any(is_hidden)
}
