use std::collections::BTreeMap;
use std::ops::Bound::{Excluded, Included};

#[derive(Clone, Debug)]
struct NaiveTime {
    year: u16,
    month: u8,
    day: u8,
    h: u8,
    m: u8,
    s: u8,
}

impl NaiveTime {
    fn new(year: u16, month: u8, day: u8, h: u8, m: u8, s: u8) -> Self {
        NaiveTime {
            year,
            month,
            day,
            h,
            m,
            s,
        }
    }
    fn date(year: u16, month: u8, day: u8) -> Self {
        NaiveTime {
            year,
            month,
            day,
            h: 0,
            m: 0,
            s: 0,
        }
    }
}

const MONTH_LENGTHS: [u8; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];

impl Into<u64> for NaiveTime {
    fn into(self) -> u64 {
        ((self.year as u64 - 1970) * 86400 * 365)
            + (86400 * MONTH_LENGTHS.iter().take(self.month as usize).map(|m| *m as u64).sum::<u64>())
            + self.day as u64 * 86400
            + self.h as u64 * 3600
            + self.m as u64 * 60
            + self.s as u64
    }
}
impl From<u64> for NaiveTime {
    fn from(v: u64) -> NaiveTime {
        let year = (v / (86400 * 365) + 1970) as u16;
        let year_day = (v % (86400 * 365)) / 86400;
        let mut day = year_day;
        let mut month = 1;
        for month_length in MONTH_LENGTHS {
            let month_length = month_length as u64;
            if day < month_length {
                break;
            }
            month += 1;
            day -= month_length;
        }
        let day = day as u8;
        let day_second = v % 86400;
        let h = (day_second / 3600) as u8;
        let m = (day_second % 3600 / 60) as u8;
        let s = (day_second % 60) as u8;
        NaiveTime {
            year,
            month,
            day,
            h,
            m,
            s,
        }
    }
}

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
    eprintln!("latest: {}", latest_date);
    let mut keep: BTreeMap<u64, T> = BTreeMap::new();
    keep.insert(latest_date, inputs.remove(&latest_date).unwrap());
    for policy in policies {
        for n in 0..policy.count as u64 {
            eprintln!("n: {}", n);
            let range = (
                latest_date - (n + 1) * policy.interval,
                latest_date - n * policy.interval,
            );
            eprintln!(
                "Block from {:?} to {:?}",
                NaiveTime::from(range.0),
                NaiveTime::from(range.1)
            );
            let range = (Excluded(range.0), Included(range.1));
            if keep.range(range).next().is_some() {
                // We're already keeping an entry that fulfils this retention block
                continue;
            }
            if let Some((&k, _)) = inputs.range(range).next_back() {
                keep.insert(k, inputs.remove(&k).unwrap());
            } else {
                eprintln!("Nothing found for block.");
            }
        }
    }
    RetentionResult { drop: inputs, keep }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        for d in 1..31 {
            items.insert(NaiveTime::date(2020, 01, d).into(), ());
            items.insert(NaiveTime::new(2020, 01, d, 04, 00, 00).into(), ());
            items.insert(NaiveTime::new(2020, 01, d, 08, 00, 00).into(), ());
            items.insert(NaiveTime::new(2020, 01, d, 12, 00, 00).into(), ());
            items.insert(NaiveTime::new(2020, 01, d, 16, 00, 00).into(), ());
            items.insert(NaiveTime::new(2020, 01, d, 20, 00, 00).into(), ());
        }
        let len_before = items.len();
        let policies = [
            PeriodicRetentionPolicy {
                interval: 86400,
                count: 3,
            },
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
        items.insert(NaiveTime::new(2020, 01, 31, 20, 30, 00).into(), ());
        let RetentionResult {
            keep: mut items,
            drop,
        } = apply(&policies, items);
        assert_eq!(drop.len(), 1);

        // Now let's advance a bit more
        items.insert(NaiveTime::date(2020, 03, 01).into(), ());
        let RetentionResult {
            keep: mut items,
            drop,
        } = apply(&policies, items);
        // There shouldn't be any retained by the daily policy, since
        // we're well beyond its timespan, but two of the old weeks
        // plus the most recent one should be kept
        for &day in items.keys() {
            eprintln!("{:?}", NaiveTime::from(day));
        }
        assert_eq!(items.len(), 3);

        panic!();
    }
}
