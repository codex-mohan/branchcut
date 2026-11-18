use branchcut::{ModifiedBefore, SizeFilter};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[test]
fn fractional_sizes_are_exact_integer_byte_thresholds() {
    let f: SizeFilter = ">=1.5MiB".parse().unwrap();
    assert!(!f.matches(1_572_863));
    assert!(f.matches(1_572_864));
    assert!("1.25KB".parse::<SizeFilter>().unwrap().matches(1_250));
    assert!("<0.5KiB".parse::<SizeFilter>().unwrap().matches(511));
    assert!("1.0B".parse::<SizeFilter>().unwrap().matches(1));
    assert!("0.1B".parse::<SizeFilter>().is_err());
    assert!("1.0000000000000000001B".parse::<SizeFilter>().is_err());
}

#[test]
fn size_fraction_overflow_is_reported_without_rounding() {
    for input in [
        "18446744073709551616B",
        "18446744073709551615.1B",
        "18446744073710TB",
        "1.12345678901234567890KiB",
        "1.B",
    ] {
        assert!(input.parse::<SizeFilter>().is_err(), "accepted {input}");
    }
}

#[test]
fn rfc3339_offsets_and_fractions_normalize_to_utc() {
    let utc: ModifiedBefore = "2024-01-01T00:00:00Z".parse().unwrap();
    let offset: ModifiedBefore = "2024-01-01T05:30:00+05:30".parse().unwrap();
    assert_eq!(utc, offset);
    let east: ModifiedBefore = "2024-01-01T00:00:00.125+01:00".parse().unwrap();
    let normalized: ModifiedBefore = "2023-12-31T23:00:00.125Z".parse().unwrap();
    assert_eq!(east, normalized);
    assert_eq!(
        "1970-01-01T00:00:00.5+00:01"
            .parse::<ModifiedBefore>()
            .unwrap(),
        ModifiedBefore::Timestamp(UNIX_EPOCH - Duration::from_millis(59_500))
    );
}

#[test]
fn unix_timestamps_support_signed_fractional_seconds() {
    assert_eq!(
        "@-0.25".parse::<ModifiedBefore>().unwrap(),
        ModifiedBefore::Timestamp(UNIX_EPOCH - Duration::from_millis(250))
    );
    assert_eq!(
        "@-1.25".parse::<ModifiedBefore>().unwrap(),
        ModifiedBefore::Timestamp(UNIX_EPOCH - Duration::from_millis(1_250))
    );
    assert_eq!(
        "@1.000000001".parse::<ModifiedBefore>().unwrap(),
        ModifiedBefore::Timestamp(UNIX_EPOCH + Duration::new(1, 1))
    );
    for bad in [
        "@",
        "@1.",
        "@1.1234567890",
        "@--1",
        "@999999999999999999999999",
    ] {
        assert!(bad.parse::<ModifiedBefore>().is_err(), "accepted {bad}");
    }
    assert!(
        SystemTime::UNIX_EPOCH
            .checked_sub(Duration::from_nanos(1))
            .is_some()
    );
}
