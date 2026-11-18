pub(crate) fn contains_bytes_with_case(
    haystack: &[u8],
    needle: &[u8],
    insensitive: bool,
    normalize_separators: bool,
) -> bool {
    !needle.is_empty()
        && haystack.windows(needle.len()).any(|window| {
            window.iter().zip(needle).all(|(&found, &wanted)| {
                let normalize = |byte| {
                    if normalize_separators && byte == b'\\' {
                        b'/'
                    } else {
                        byte
                    }
                };
                let found = normalize(found);
                let wanted = normalize(wanted);
                found == wanted || (insensitive && found.eq_ignore_ascii_case(&wanted))
            })
        })
}
