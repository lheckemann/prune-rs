use std::collections::BTreeMap;

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
    for policy in policies {
        for n in 0..policy.count as u64 {
            eprintln!("n: {}", n);
            let after = latest_date - (n + 1) * policy.interval;
            let before = latest_date - n * policy.interval;
            eprintln!("Block from {} to {}", after, before);
            if keep.range(after..Included(before)).next().is_some() {
                // We're already keeping an entry that fulfils this retention block
                continue;
            }
            if let Some((&k, _)) = inputs.range(after..before).next_back() {
                keep.insert(k, inputs.remove(&k).unwrap());
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
}
