/* Copyright (C) 2022 Linus Heckemann. Licensed under the EUPL-1.2-or-later. */

use std::collections::BTreeMap;
use std::ops::Bound::{Excluded, Included};

use chrono::NaiveDateTime;
use clap::{command, Arg};

#[derive(Debug, Clone)]
struct PeriodicRetentionPolicy {
    interval: u64,
    count: u32,
}

struct RetentionResult<T> {
    pub keep: BTreeMap<u64, T>,
    pub drop: BTreeMap<u64, T>,
}

fn apply<T>(
    policies: &[PeriodicRetentionPolicy],
    mut inputs: BTreeMap<u64, T>,
) -> RetentionResult<T>
where
    T: Clone,
{
    let latest_date = match inputs.iter().next_back() {
        None => {
            return RetentionResult {
                keep: BTreeMap::new(),
                drop: BTreeMap::new(),
            }
        } // Empty input, nothing to do.
        Some((&date, _)) => date,
    };
    let mut keep: BTreeMap<u64, T> = BTreeMap::new();
    keep.insert(latest_date, inputs.remove(&latest_date).unwrap());
    for policy in policies {
        let reference_date = latest_date - (latest_date % policy.interval) + policy.interval;
        for n in 0..(policy.count + 1) as u64 {
            let range = (
                reference_date - (n + 1) * policy.interval,
                reference_date - n * policy.interval,
            );
            let range = (Excluded(range.0), Included(range.1));
            if keep.range(range).next().is_some() {
                // We're already keeping an entry that fulfils this retention block
                continue;
            }
            if let Some((&k, _)) = inputs.range(range).next_back() {
                keep.insert(k, inputs.remove(&k).unwrap());
            }
        }
    }
    RetentionResult { drop: inputs, keep }
}

fn main() -> std::io::Result<()> {
    let args = command!()
        .arg(
            Arg::new("policy")
                .short('p')
                .help("Define a periodic retention policy")
                .required(true)
                .multiple_occurrences(true)
                .number_of_values(2)
                .value_names(&["duration", "count"]),
        )
        .arg(
            Arg::new("format")
                .short('f')
                .help("Format to parse the date strings on stdin with")
                .default_value("%Y%m%d-%H:%M"),
        )
        .arg(
            Arg::new("verbose")
                .short('v')
                .help("Print the snapshots which are being kept on stderr"),
        )
        .get_matches();
    let mut policy_defs = args.values_of("policy").unwrap();
    let mut policies = Vec::new();
    while let (Some(interval), Some(count)) = (policy_defs.next(), policy_defs.next()) {
        policies.push(PeriodicRetentionPolicy {
            interval: u64::from_str_radix(interval, 10)
                .unwrap_or_else(|_| panic!("Invalid interval '{}'", interval)),
            count: u32::from_str_radix(count, 10)
                .unwrap_or_else(|_| panic!("Invalid count '{}'", count)),
        })
    }
    let mut snaps = BTreeMap::new();
    let stdin = std::io::stdin();
    let mut line = String::new();
    while let Ok(len) = stdin.read_line(&mut line) {
        if len == 0 {
            break;
        }
        line = line.trim_end().into();
        if let Ok(date) = NaiveDateTime::parse_from_str(&line, args.value_of("format").unwrap()) {
            snaps.insert(date.timestamp() as u64, line.clone());
        } else {
            eprintln!("Could not parse line: '{}'", &line);
        }
        line.clear();
    }
    let RetentionResult { keep, drop } = apply(&policies, snaps);
    for snap in drop.iter() {
        println!("{}", snap.1);
    }
    if args.is_present("verbose") {
        for snap in keep.iter() {
            eprintln!("Keep {}", snap.1);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use chrono::{NaiveDate, NaiveTime};

    use super::*;

    /// Parse a date into a NaiveDateTime according to a fixed format and return the timestamp as a u64.
    fn date(string: &str) -> u64 {
        NaiveDateTime::parse_from_str(string, "%Y-%m-%dT%H:%M:%S")
            .unwrap()
            .timestamp() as u64
    }

    #[test]
    fn test_keep_single() {
        let mut items = BTreeMap::new();
        items.insert(3, ());
        let RetentionResult { keep, drop } = apply(
            &[PeriodicRetentionPolicy {
                interval: 1,
                count: 1,
            }],
            items,
        );
        assert_eq!(drop.len(), 0);
        assert_eq!(keep.len(), 1);
        assert_eq!(keep.keys().next(), Some(&3u64));
    }

    #[test]
    fn test_keep_multiple() {
        let mut items = BTreeMap::new();
        let times: Vec<NaiveTime> = (0..5).map(|n| NaiveTime::from_hms(n * 5, 0, 0)).collect();
        for d in 1..31 {
            for time in times.iter() {
                items.insert(
                    NaiveDateTime::new(NaiveDate::from_ymd(2020, 01, d), *time).timestamp() as u64,
                    (),
                );
            }
        }
        let len_before = items.len();
        let policies = [
            // daily snapshots for a week
            PeriodicRetentionPolicy {
                interval: 86400,
                count: 3,
            },
            // weekly snapshots for 6 weeks
            PeriodicRetentionPolicy {
                interval: 86400 * 7,
                count: 6,
            },
        ];
        let RetentionResult { keep, drop } = apply(&policies, items);
        assert_ne!(drop.len(), 0);
        assert_eq!(drop.len() + keep.len(), len_before);
        // Idempotency
        let RetentionResult {
            keep: mut items,
            drop,
        } = apply(&policies, keep);
        assert_eq!(drop.len(), 0);

        // We should only drop one snapshot if we get a slightly more recent one available
        items.insert(date("2020-01-31T20:30:00"), ());
        let RetentionResult {
            keep: mut items,
            drop,
        } = apply(&policies, items);
        assert_eq!(drop.len(), 1);

        // Now let's advance a bit more
        items.insert(date("2020-03-01T12:00:00"), ());
        let RetentionResult {
            keep: mut items,
            drop,
        } = apply(&policies, items);
        // There shouldn't be any retained by the daily policy, since
        // we're well beyond its timespan, but two or three from older weeks
        // plus the most recent one should be kept
        for &day in items.keys() {
            eprintln!("{:?}", NaiveDateTime::from_timestamp(day as i64, 0));
        }
        assert!(3 <= items.len() && items.len() <= 4);
    }
}
