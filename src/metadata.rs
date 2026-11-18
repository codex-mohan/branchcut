//! Size and modification-time predicates used by filesystem queries.
//!
//! Durations use fixed units (`ms`, `s`, `m`, `h`, `d`, `w`); calendar months
//! and years are intentionally unsupported. Absolute timestamps accept UTC,
//! RFC3339 offsets, and Unix `@seconds` notation.

use crate::QueryError;
use std::fmt;
use std::fs::Metadata;
use std::io;
use std::str::FromStr;
use std::time::{Duration, SystemTime};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Comparison {
    Greater,
    GreaterOrEqual,
    Less,
    LessOrEqual,
    Equal,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SizeFilter {
    comparison: Comparison,
    bytes: u64,
}

impl SizeFilter {
    pub fn matches(&self, bytes: u64) -> bool {
        match self.comparison {
            Comparison::Greater => bytes > self.bytes,
            Comparison::GreaterOrEqual => bytes >= self.bytes,
            Comparison::Less => bytes < self.bytes,
            Comparison::LessOrEqual => bytes <= self.bytes,
            Comparison::Equal => bytes == self.bytes,
        }
    }
}

impl FromStr for SizeFilter {
    type Err = QueryError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let input = input.trim();
        if input.is_empty() {
            return Err(QueryError("size filter cannot be empty".into()));
        }
        let (comparison, rest) = if let Some(rest) = input.strip_prefix(">=") {
            (Comparison::GreaterOrEqual, rest)
        } else if let Some(rest) = input.strip_prefix("<=") {
            (Comparison::LessOrEqual, rest)
        } else if let Some(rest) = input.strip_prefix('>') {
            (Comparison::Greater, rest)
        } else if let Some(rest) = input.strip_prefix('<') {
            (Comparison::Less, rest)
        } else if let Some(rest) = input.strip_prefix('=') {
            (Comparison::Equal, rest)
        } else {
            (Comparison::Equal, input)
        };
        let rest = rest.trim_start();
        if rest.starts_with('-') {
            return Err(QueryError("size filter cannot be negative".into()));
        }
        let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
        if digits == 0 {
            return Err(QueryError(format!(
                "size filter {input:?} must start with a whole number"
            )));
        }
        let mut number_end = digits;
        if rest.as_bytes().get(number_end) == Some(&b'.') {
            number_end += 1;
            let fraction_start = number_end;
            number_end += rest[number_end..]
                .bytes()
                .take_while(u8::is_ascii_digit)
                .count();
            if number_end == fraction_start {
                return Err(QueryError(
                    "fractional size requires digits after the decimal point".into(),
                ));
            }
        }
        let number = &rest[..number_end];
        let unit = rest[number_end..].trim();
        let multiplier = size_multiplier(unit)
            .ok_or_else(|| QueryError(format!("unknown size unit {unit:?}")))?;
        let bytes = exact_size_bytes(number, multiplier)?;
        Ok(Self { comparison, bytes })
    }
}

fn exact_size_bytes(number: &str, multiplier: u64) -> Result<u64, QueryError> {
    let (whole, fraction) = number.split_once('.').unwrap_or((number, ""));
    let whole = whole
        .parse::<u64>()
        .map_err(|_| QueryError("size value exceeds the supported u64 range".into()))?;
    let whole_bytes = whole
        .checked_mul(multiplier)
        .ok_or_else(|| QueryError("size value overflows the supported u64 byte range".into()))?;
    if fraction.is_empty() {
        return Ok(whole_bytes);
    }
    let numerator = fraction
        .parse::<u128>()
        .map_err(|_| QueryError("size fraction is out of range".into()))?;
    let denominator = 10_u128
        .checked_pow(fraction.len() as u32)
        .ok_or_else(|| QueryError("size fraction has too many digits".into()))?;
    let scaled = numerator
        .checked_mul(u128::from(multiplier))
        .ok_or_else(|| QueryError("size value overflows the supported u64 byte range".into()))?;
    if scaled % denominator != 0 {
        return Err(QueryError(
            "size threshold must resolve to a whole number of bytes".into(),
        ));
    }
    let fractional_bytes = u64::try_from(scaled / denominator)
        .map_err(|_| QueryError("size value overflows the supported u64 byte range".into()))?;
    whole_bytes
        .checked_add(fractional_bytes)
        .ok_or_else(|| QueryError("size value overflows the supported u64 byte range".into()))
}

fn size_multiplier(unit: &str) -> Option<u64> {
    Some(match unit {
        "" | "B" => 1,
        "K" | "kB" | "KB" => 1_000,
        "M" | "MB" => 1_000_000,
        "G" | "GB" => 1_000_000_000,
        "T" | "TB" => 1_000_000_000_000,
        "KiB" => 1 << 10,
        "MiB" => 1 << 20,
        "GiB" => 1 << 30,
        "TiB" => 1 << 40,
        _ => return None,
    })
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModifiedBefore {
    /// Match paths modified strictly before `now - age`.
    Age(Duration),
    /// Match paths modified strictly before this absolute instant.
    Timestamp(SystemTime),
}

impl FromStr for ModifiedBefore {
    type Err = QueryError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let input = input.trim();
        if let Some(timestamp) = input.strip_prefix('@') {
            parse_unix_timestamp(timestamp).map(Self::Timestamp)
        } else if input.contains('T') {
            parse_utc_timestamp(input).map(Self::Timestamp)
        } else if input.len() == 10 && input.as_bytes().get(4) == Some(&b'-') {
            parse_utc_date(input).map(Self::Timestamp)
        } else {
            let age = parse_duration(input)?;
            Ok(Self::Age(age))
        }
    }
}

/// Parse a nonnegative whole-number duration with one fixed unit.
pub fn parse_duration(input: &str) -> Result<Duration, QueryError> {
    let input = input.trim();
    if input.is_empty() {
        return Err(QueryError("duration cannot be empty".into()));
    }
    if input.starts_with('-') {
        return Err(QueryError("duration cannot be negative".into()));
    }
    let digits = input.bytes().take_while(u8::is_ascii_digit).count();
    if digits == 0 {
        return Err(QueryError(format!(
            "duration {input:?} must start with a whole number"
        )));
    }
    let value = input[..digits]
        .parse::<u64>()
        .map_err(|_| QueryError("duration value exceeds the supported u64 range".into()))?;
    let unit = input[digits..].trim();
    let nanos_per_unit: u128 = match unit {
        "ms" => 1_000_000,
        "s" => 1_000_000_000,
        "m" => 60 * 1_000_000_000,
        "h" => 60 * 60 * 1_000_000_000,
        "d" => 24 * 60 * 60 * 1_000_000_000,
        "w" => 7 * 24 * 60 * 60 * 1_000_000_000,
        _ => {
            return Err(QueryError(format!(
                "unknown duration unit {unit:?}; use ms, s, m, h, d, or w"
            )));
        }
    };
    let nanos = u128::from(value)
        .checked_mul(nanos_per_unit)
        .ok_or_else(|| QueryError("duration value overflows the supported range".into()))?;
    duration_from_nanos(nanos)
}

fn duration_from_nanos(nanos: u128) -> Result<Duration, QueryError> {
    let seconds = nanos / 1_000_000_000;
    if seconds > u128::from(u64::MAX) {
        return Err(QueryError(
            "duration value overflows the supported range".into(),
        ));
    }
    Ok(Duration::new(
        seconds as u64,
        (nanos % 1_000_000_000) as u32,
    ))
}

fn parse_utc_date(input: &str) -> Result<SystemTime, QueryError> {
    let bytes = input.as_bytes();
    if bytes.len() != 10 || !bytes.is_ascii() || bytes[4] != b'-' || bytes[7] != b'-' {
        return Err(QueryError("date must use YYYY-MM-DD format".into()));
    }
    let year = parse_decimal(&input[0..4], "year")?;
    let month = parse_decimal(&input[5..7], "month")?;
    let day = parse_decimal(&input[8..10], "day")?;
    if !(1970..=9999).contains(&year) {
        return Err(QueryError(
            "UTC date year must be between 1970 and 9999".into(),
        ));
    }
    if !(1..=12).contains(&month) {
        return Err(QueryError(
            "UTC date month must be between 01 and 12".into(),
        ));
    }
    let days_in_month = match month {
        2 if is_leap_year(year) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    };
    if !(1..=days_in_month).contains(&day) {
        return Err(QueryError(format!(
            "invalid day {day:02} for month {month:02}"
        )));
    }
    let days = days_from_civil(year, month, day);
    system_time_from_seconds(u64::try_from(days * 86_400).unwrap())
}

fn parse_utc_timestamp(input: &str) -> Result<SystemTime, QueryError> {
    let bytes = input.as_bytes();
    if !bytes.is_ascii()
        || bytes.len() < 20
        || bytes.get(10) != Some(&b'T')
        || bytes.get(13) != Some(&b':')
        || bytes.get(16) != Some(&b':')
    {
        return Err(QueryError(
            "timestamp must use RFC3339 YYYY-MM-DDTHH:MM:SS[.fraction](Z|±HH:MM) format".into(),
        ));
    }
    let date = parse_utc_date(&input[..10])?;
    let hour = parse_decimal(&input[11..13], "hour")?;
    let minute = parse_decimal(&input[14..16], "minute")?;
    let second = parse_decimal(&input[17..19], "second")?;
    if hour > 23 || minute > 59 || second > 59 {
        return Err(QueryError(
            "timestamp has an out-of-range time component".into(),
        ));
    }
    let mut cursor = 19;
    let nanos = if bytes.get(cursor) == Some(&b'.') {
        cursor += 1;
        let start = cursor;
        while bytes.get(cursor).is_some_and(u8::is_ascii_digit) {
            cursor += 1;
        }
        let digits = cursor - start;
        if digits == 0 || digits > 9 {
            return Err(QueryError(
                "timestamp fraction must contain 1 to 9 digits".into(),
            ));
        }
        let value = input[start..cursor]
            .parse::<u32>()
            .map_err(|_| QueryError("invalid timestamp fraction".into()))?;
        value * 10_u32.pow((9 - digits) as u32)
    } else {
        0
    };
    let offset_seconds: i64 = match bytes.get(cursor) {
        Some(b'Z') if cursor + 1 == bytes.len() => 0,
        Some(sign @ (b'+' | b'-'))
            if cursor + 6 == bytes.len() && bytes.get(cursor + 3) == Some(&b':') =>
        {
            let oh = parse_decimal(&input[cursor + 1..cursor + 3], "offset hour")?;
            let om = parse_decimal(&input[cursor + 4..cursor + 6], "offset minute")?;
            if oh > 23 || om > 59 {
                return Err(QueryError("timestamp offset is out of range".into()));
            }
            let magnitude = i64::from(oh * 3600 + om * 60);
            if *sign == b'+' { magnitude } else { -magnitude }
        }
        _ => {
            return Err(QueryError(
                "timestamp must end in Z or a numeric ±HH:MM offset".into(),
            ));
        }
    };
    let local_seconds = date
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("validated dates are at or after the Unix epoch")
        .as_secs() as i64
        + i64::from(hour * 3600 + minute * 60 + second);
    system_time_from_signed_nanos(
        i128::from(local_seconds - offset_seconds) * 1_000_000_000 + i128::from(nanos),
    )
}

fn parse_unix_timestamp(input: &str) -> Result<SystemTime, QueryError> {
    if input.is_empty() {
        return Err(QueryError("Unix timestamp after @ cannot be empty".into()));
    }
    let (negative, unsigned) = if let Some(s) = input.strip_prefix('-') {
        (true, s)
    } else if let Some(s) = input.strip_prefix('+') {
        (false, s)
    } else {
        (false, input)
    };
    let (whole, fraction) = unsigned.split_once('.').unwrap_or((unsigned, ""));
    if whole.is_empty()
        || !whole.bytes().all(|b| b.is_ascii_digit())
        || (!fraction.is_empty() && !fraction.bytes().all(|b| b.is_ascii_digit()))
        || (unsigned.contains('.') && fraction.is_empty())
    {
        return Err(QueryError(
            "Unix timestamp must be @seconds with an optional decimal fraction".into(),
        ));
    }
    if fraction.len() > 9 {
        return Err(QueryError(
            "Unix timestamp fraction supports at most 9 digits".into(),
        ));
    }
    let seconds = whole
        .parse::<i128>()
        .map_err(|_| QueryError("Unix timestamp exceeds the supported range".into()))?;
    let fraction_value = if fraction.is_empty() {
        0
    } else {
        fraction
            .parse::<i128>()
            .map_err(|_| QueryError("invalid Unix timestamp fraction".into()))?
            * 10_i128.pow((9 - fraction.len()) as u32)
    };
    let magnitude = seconds
        .checked_mul(1_000_000_000)
        .and_then(|n| n.checked_add(fraction_value))
        .ok_or_else(|| QueryError("Unix timestamp exceeds the supported range".into()))?;
    system_time_from_signed_nanos(if negative { -magnitude } else { magnitude })
}

fn system_time_from_signed_nanos(nanos: i128) -> Result<SystemTime, QueryError> {
    let magnitude = nanos.unsigned_abs();
    let seconds = u64::try_from(magnitude / 1_000_000_000)
        .map_err(|_| QueryError("timestamp exceeds the supported range".into()))?;
    let duration = Duration::new(seconds, (magnitude % 1_000_000_000) as u32);
    if nanos < 0 {
        SystemTime::UNIX_EPOCH.checked_sub(duration)
    } else {
        SystemTime::UNIX_EPOCH.checked_add(duration)
    }
    .ok_or_else(|| QueryError("timestamp exceeds the platform time range".into()))
}

fn parse_decimal(value: &str, component: &str) -> Result<u32, QueryError> {
    if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(QueryError(format!("UTC {component} must contain digits")));
    }
    value
        .parse::<u32>()
        .map_err(|_| QueryError(format!("UTC {component} is out of range")))
}

fn is_leap_year(year: u32) -> bool {
    year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400))
}

/// Days since 1970-01-01, for validated Gregorian calendar components.
fn days_from_civil(year: u32, month: u32, day: u32) -> i64 {
    let years = (1970..year)
        .map(|year| if is_leap_year(year) { 366 } else { 365 })
        .sum::<i64>();
    let months = (1..month)
        .map(|month| match month {
            2 if is_leap_year(year) => 29,
            2 => 28,
            4 | 6 | 9 | 11 => 30,
            _ => 31,
        })
        .sum::<i64>();
    years + months + i64::from(day) - 1
}

fn system_time_from_seconds(seconds: u64) -> Result<SystemTime, QueryError> {
    SystemTime::UNIX_EPOCH
        .checked_add(Duration::from_secs(seconds))
        .ok_or_else(|| QueryError("UTC timestamp exceeds the platform time range".into()))
}

#[derive(Clone, Debug, Default)]
pub struct MetadataFilters {
    pub sizes: Vec<SizeFilter>,
    pub modified_within: Option<Duration>,
    pub modified_before: Option<ModifiedBefore>,
}

impl MetadataFilters {
    /// Reject unrepresentable relative boundaries before filesystem traversal.
    pub(crate) fn validate_time(&self, now: SystemTime) -> io::Result<()> {
        for age in [
            self.modified_within,
            match self.modified_before {
                Some(ModifiedBefore::Age(age)) => Some(age),
                _ => None,
            },
        ]
        .into_iter()
        .flatten()
        {
            if now.checked_sub(age).is_none() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "modification-time duration exceeds the platform time range",
                ));
            }
        }
        Ok(())
    }
    pub fn needs_metadata(&self) -> bool {
        !self.sizes.is_empty() || self.modified_within.is_some() || self.modified_before.is_some()
    }

    /// Evaluate all predicates against already-fetched metadata.
    pub fn matches(&self, metadata: &Metadata, now: SystemTime) -> io::Result<bool> {
        if self
            .sizes
            .iter()
            .any(|filter| !filter.matches(metadata.len()))
        {
            return Ok(false);
        }
        if self.modified_within.is_none() && self.modified_before.is_none() {
            return Ok(true);
        }
        let modified = metadata.modified()?;
        if let Some(within) = self.modified_within {
            let threshold = now.checked_sub(within).ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "modified-within duration exceeds the representable time range",
                )
            })?;
            if modified < threshold {
                return Ok(false);
            }
        }
        if let Some(before) = self.modified_before {
            let threshold = match before {
                ModifiedBefore::Timestamp(timestamp) => timestamp,
                ModifiedBefore::Age(age) => now.checked_sub(age).ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "modified-before age exceeds the representable time range",
                    )
                })?,
            };
            if modified >= threshold {
                return Ok(false);
            }
        }
        Ok(true)
    }
}

impl fmt::Display for SizeFilter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let op = match self.comparison {
            Comparison::Greater => ">",
            Comparison::GreaterOrEqual => ">=",
            Comparison::Less => "<",
            Comparison::LessOrEqual => "<=",
            Comparison::Equal => "=",
        };
        write!(f, "{op}{}B", self.bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::UNIX_EPOCH;

    #[test]
    fn size_filter_comparisons_and_units() {
        assert!("10".parse::<SizeFilter>().unwrap().matches(10));
        assert!(!"10".parse::<SizeFilter>().unwrap().matches(9));
        assert!(">10B".parse::<SizeFilter>().unwrap().matches(11));
        assert!(">=2KB".parse::<SizeFilter>().unwrap().matches(2_000));
        assert!("<2kB".parse::<SizeFilter>().unwrap().matches(1_999));
        assert!("<=1KiB".parse::<SizeFilter>().unwrap().matches(1_024));
        assert!("1M".parse::<SizeFilter>().unwrap().matches(1_000_000));
        assert!("1TiB".parse::<SizeFilter>().unwrap().matches(1_u64 << 40));
    }

    #[test]
    fn size_filter_rejects_invalid_and_overflowing_values() {
        for input in [
            "",
            "-1",
            "1.5B",
            "1MBx",
            "1KBx",
            "18446744073709551616",
            "18446744073710TB",
        ] {
            assert!(input.parse::<SizeFilter>().is_err(), "accepted {input:?}");
        }
    }

    #[test]
    fn durations_require_whole_values_and_fixed_units() {
        assert_eq!(parse_duration("0ms").unwrap(), Duration::ZERO);
        assert_eq!(
            parse_duration("2w").unwrap(),
            Duration::from_secs(1_209_600)
        );
        assert_eq!(parse_duration("3h").unwrap(), Duration::from_secs(10_800));
        for input in ["-1d", "1.5h", "1", "2mo", "1y", "18446744073709551615w"] {
            assert!(parse_duration(input).is_err(), "accepted {input:?}");
        }
    }

    #[test]
    fn utc_dates_validate_leap_years_and_time_components() {
        let leap = "2024-02-29".parse::<ModifiedBefore>().unwrap();
        assert_eq!(
            leap,
            ModifiedBefore::Timestamp(UNIX_EPOCH + Duration::from_secs(1_709_164_800))
        );
        assert!("2023-02-29".parse::<ModifiedBefore>().is_err());
        assert!("1900-02-29".parse::<ModifiedBefore>().is_err());
        assert!("2000-02-29".parse::<ModifiedBefore>().is_ok());
        assert!("2100-02-29".parse::<ModifiedBefore>().is_err());
        assert!("2024-13-01".parse::<ModifiedBefore>().is_err());
        assert!("2024-01-01T24:00:00Z".parse::<ModifiedBefore>().is_err());
        assert!("2024-01-01T00:00:60Z".parse::<ModifiedBefore>().is_err());
        assert_eq!(
            "0d".parse::<ModifiedBefore>().unwrap(),
            ModifiedBefore::Age(Duration::ZERO)
        );
        assert!("1969-12-31".parse::<ModifiedBefore>().is_err());
    }

    #[test]
    fn utc_timestamp_is_parsed_without_local_timezone() {
        let parsed = "1970-01-02T03:04:05Z".parse::<ModifiedBefore>().unwrap();
        assert_eq!(
            parsed,
            ModifiedBefore::Timestamp(UNIX_EPOCH + Duration::from_secs(97_445))
        );
        assert_eq!(
            "2024-01-01T00:00:00+00:00"
                .parse::<ModifiedBefore>()
                .unwrap(),
            "2024-01-01T00:00:00Z".parse::<ModifiedBefore>().unwrap()
        );
    }

    #[test]
    fn metadata_filters_and_and_boundary_semantics() {
        let path = std::env::temp_dir().join(format!("branchcut-metadata-{}", std::process::id()));
        fs::write(&path, b"1234").unwrap();
        let metadata = fs::metadata(&path).unwrap();
        let modified = metadata.modified().unwrap();
        let filters = MetadataFilters {
            sizes: vec!["4".parse().unwrap()],
            modified_within: Some(Duration::ZERO),
            modified_before: Some(ModifiedBefore::Timestamp(modified + Duration::from_secs(1))),
        };
        assert!(filters.needs_metadata());
        assert!(filters.matches(&metadata, modified).unwrap());

        let strict_before = MetadataFilters {
            modified_before: Some(ModifiedBefore::Timestamp(modified)),
            ..MetadataFilters::default()
        };
        assert!(!strict_before.matches(&metadata, modified).unwrap());

        let inclusive_within = MetadataFilters {
            modified_within: Some(Duration::ZERO),
            ..MetadataFilters::default()
        };
        assert!(inclusive_within.matches(&metadata, modified).unwrap());

        fs::remove_file(path).unwrap();
        assert!(!MetadataFilters::default().needs_metadata());
    }

    #[test]
    fn huge_age_reports_underflow_as_io_error() {
        let path =
            std::env::temp_dir().join(format!("branchcut-metadata-age-{}", std::process::id()));
        fs::write(&path, b"x").unwrap();
        let metadata = fs::metadata(&path).unwrap();
        let filters = MetadataFilters {
            modified_within: Some(Duration::from_secs(u64::MAX)),
            ..MetadataFilters::default()
        };
        let result = filters.matches(&metadata, UNIX_EPOCH);
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::InvalidInput);
        fs::remove_file(path).unwrap();
    }
}
