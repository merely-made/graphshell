// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

use super::*;

#[test]
fn a_date_reads_and_knows_its_calendar() {
    assert_eq!(
        parse_date("2024-02-29"),
        Ok(Date {
            year: 2024,
            month: 2,
            day: 29
        })
    );
    for bad in [
        "2023-02-29",
        "1900-02-29",
        "2026-13-01",
        "2026-04-31",
        "2026-00-10",
        "2026-1-01",
        "26-01-01",
        "2026-01-01Z",
        "2026/01/01",
        "",
    ] {
        assert!(parse_date(bad).is_err(), "{bad:?} should be refused");
    }
    assert!(parse_date("2000-02-29").is_ok());
}

#[test]
fn a_date_time_names_one_instant() {
    let utc = parse_date_time("1970-01-01T00:00:00Z").unwrap();
    assert_eq!(utc.seconds, 0);
    assert_eq!(
        parse_date_time("2026-11-19T12:00:00Z"),
        parse_date_time("2026-11-19T13:00:00+01:00")
    );
    assert_eq!(
        parse_date_time("2026-11-19T00:30:00-01:00").unwrap(),
        parse_date_time("2026-11-19T01:30:00Z").unwrap()
    );
    // Hinnant's algorithm against a known stamp: 2000-03-01 is day 11017.
    assert_eq!(
        parse_date_time("2000-03-01T00:00:00Z").unwrap().seconds,
        11_017 * 86_400
    );
}

#[test]
fn fractions_compare_as_numbers() {
    let at = |text| parse_date_time(text).unwrap();
    assert!(at("2026-01-01T00:00:00.5Z") > at("2026-01-01T00:00:00.25Z"));
    assert!(at("2026-01-01T00:00:00.05Z") < at("2026-01-01T00:00:00.5Z"));
    assert_eq!(at("2026-01-01T00:00:00.500Z"), at("2026-01-01T00:00:00.5Z"));
    assert_eq!(at("2026-01-01T00:00:00.000Z"), at("2026-01-01T00:00:00Z"));
}

#[test]
fn a_date_time_outside_both_standards_is_refused() {
    for bad in [
        "2026-11-19T12:00:00",  // no offset: names no instant
        "2026-11-19t12:00:00Z", // RFC 3339 allows lower case; XSD does not
        "2026-11-19T12:00:00z",
        "2026-11-19 12:00:00Z",
        "2026-11-19T24:00:00Z",      // XSD allows 24:00:00; RFC 3339 does not
        "2026-11-19T12:00:60Z",      // a leap second; XSD does not allow it
        "2026-11-19T12:00:00+15:00", // XSD limits offsets to fourteen hours
        "2026-11-19T12:00:00+14:30",
        "2026-11-19T12:00:00.Z",
        "2026-11-19T12:00Z",
        "2026-02-30T12:00:00Z",
        "2026-11-19T12:00:00+0100",
    ] {
        assert!(parse_date_time(bad).is_err(), "{bad:?} should be refused");
    }
    assert!(parse_date_time("2026-11-19T12:00:00+14:00").is_ok());
    assert!(parse_date_time("2026-11-19T12:00:00-00:00").is_ok());
}
