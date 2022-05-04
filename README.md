# Decision maker for snapshot pruning

⚠ THIS IS NOT THOROUGHLY TESTED. USE IT AT YOUR OWN RISK. ⚠

I use it personally. If you choose to use it and it results in
deleting all your data, please tell me, but I refuse any blame!

Use case: you take filesystem snapshots using a fancy filesystem like
zfs or btrfs on a frequent basis (e.g. every 5 minutes), but don't
want to keep hundreds of snapshots per day forever. This tool takes a
list of snapshots on standard input, and outputs the ones that should
be dropped, according to retention policies passed on the command
line:

```
$ ls -d /btrfs/snaps/safe-* |
    prune-rs \
        -p 300 6 $(: keep snapshots from the last six 5-minute periods) \
        -p 3600 24 $(: and from the last 24 hours) \
        -p 86400 7 $(: and the last 7 days) \
        -p $((86400*7)) 6 $(: and the last 6 weeks) \
        -f /btrfs/snaps/safe-%Y%m%d-%H:%M # and parse the dates according to this format
/btrfs/snaps/safe-20220504-13:45
/btrfs/snaps/safe-20220504-13:50
/btrfs/snaps/safe-20220504-13:55
/btrfs/snaps/safe-20220504-14:05
/btrfs/snaps/safe-20220504-14:10
/btrfs/snaps/safe-20220504-14:15
/btrfs/snaps/safe-20220504-14:20
Keep /btrfs/snaps/safe-20220420-12:20
Keep /btrfs/snaps/safe-20220427-22:40
Keep /btrfs/snaps/safe-20220428-20:10
Keep /btrfs/snaps/safe-20220429-17:45
Keep /btrfs/snaps/safe-20220501-00:00
Keep /btrfs/snaps/safe-20220502-00:00
Keep /btrfs/snaps/safe-20220502-23:35
Keep /btrfs/snaps/safe-20220503-15:00
Keep /btrfs/snaps/safe-20220503-16:00
Keep /btrfs/snaps/safe-20220503-17:00
Keep /btrfs/snaps/safe-20220503-18:00
Keep /btrfs/snaps/safe-20220503-19:00
Keep /btrfs/snaps/safe-20220503-20:00
Keep /btrfs/snaps/safe-20220503-21:00
Keep /btrfs/snaps/safe-20220503-22:00
Keep /btrfs/snaps/safe-20220503-22:35
Keep /btrfs/snaps/safe-20220504-10:00
Keep /btrfs/snaps/safe-20220504-11:00
Keep /btrfs/snaps/safe-20220504-12:00
Keep /btrfs/snaps/safe-20220504-13:00
Keep /btrfs/snaps/safe-20220504-14:00
Keep /btrfs/snaps/safe-20220504-14:25
Keep /btrfs/snaps/safe-20220504-14:30
Keep /btrfs/snaps/safe-20220504-14:35
Keep /btrfs/snaps/safe-20220504-14:40
Keep /btrfs/snaps/safe-20220504-14:45
Keep /btrfs/snaps/safe-20220504-14:50
```

This can be used in a script, e.g. the one that produces the snapshots. Mine looks like this, and runs every 5 minutes:
```bash
btrfs subvolume snapshot -r /btrfs/safe /btrfs/snaps/safe-"$(date +%Y%m%d-%H:%M)"
printf "%s\n" /btrfs/snaps/safe-* |
    prune-rs \
        -f /btrfs/snaps/safe-%Y%m%d-%H:%M \
        -p 300 6 \
        -p 3600 24 \
        -p 86400 7 \
        -p $((86400*7)) 6 |
    xargs -r btrfs subvolume delete
```
