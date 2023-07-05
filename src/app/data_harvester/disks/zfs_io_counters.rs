use crate::app::data_harvester::disks::IoCounters;

/// Returns zpool I/O stats. Pulls data from `sysctl kstat.zfs.{POOL}.dataset.{objset-*}`
#[cfg(target_os = "freebsd")]
pub fn zfs_io_stats() -> anyhow::Result<Vec<anyhow::Result<IoCounters>>> {
    use sysctl::Sysctl;
    let zfs_ctls: Vec<_> = sysctl::Ctl::new("kstat.zfs.")?
        .into_iter()
        .filter_map(|e| {
            e.ok().and_then(|ctl| {
                let name = ctl.name();
                if let Ok(name) = name {
                    if name.contains("objset-") {
                        if name.contains("dataset_name")
                            || name.contains("nwritten")
                            || name.contains("nread")
                        {
                            Some(ctl)
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
        })
        .collect();

    use itertools::Itertools;
    let results: Vec<anyhow::Result<IoCounters>> = zfs_ctls
        .iter()
        .chunks(3)
        .into_iter()
        .filter_map(|chunk| {
            let mut nread = 0;
            let mut nwrite = 0;
            let mut ds_name = String::new();
            for ctl in chunk {
                if let Ok(name) = ctl.name() {
                    if name.contains("dataset_name") {
                        ds_name = ctl.value_string().ok()?;
                    } else if name.contains("nread") {
                        if let Ok(sysctl::CtlValue::U64(val)) = ctl.value() {
                            nread = val;
                        }
                    } else if name.contains("nwritten") {
                        if let Ok(sysctl::CtlValue::U64(val)) = ctl.value() {
                            nwrite = val;
                        }
                    }
                }
            }
            Some(Ok(IoCounters::new(ds_name.to_string(), nread, nwrite)))
        })
        .collect();
    Ok(results)
}

/// Returns zpool I/O stats. Pulls data from `/proc/spl/kstat/zfs/*/objset-*`.
#[cfg(target_os = "linux")]
pub fn zfs_io_stats() -> anyhow::Result<Vec<anyhow::Result<IoCounters>>> {
    let mut results = vec![];
    if let Ok(zpools) = std::fs::read_dir("/proc/spl/kstat/zfs") {
        let zpools_vec: Vec<std::path::PathBuf> = zpools
            .filter_map(|e| {
                e.ok().and_then(|d| {
                    let p = d.path();
                    if p.is_dir() {
                        Some(p)
                    } else {
                        None
                    }
                })
            })
            .collect();

        zpools_vec.iter().for_each(|zpool| {
            if let Ok(datasets) = std::fs::read_dir(zpool) {
                let datasets_vec: Vec<std::path::PathBuf> = datasets
                    .filter_map(|e| {
                        e.ok().and_then(|d| {
                            let p = d.path();
                            if p.is_file() && p.to_str()?.contains("objset-") {
                                Some(p)
                            } else {
                                None
                            }
                        })
                    })
                    .collect();
                for dataset in datasets_vec {
                    // TODO: [OPT] is this efficient?
                    if let Ok(contents) = std::fs::read_to_string(dataset) {
                        let mut nread = 0;
                        let mut nwritten = 0;
                        let mut ds_name = "";
                        for line in contents.lines() {
                            if let Some((label, value)) = line.split_once(' ') {
                                match label {
                                    "dataset_name" => {
                                        if let Some((_type, val)) =
                                            value.trim_start().rsplit_once(' ')
                                        {
                                            ds_name = val;
                                        }
                                    }
                                    "nwritten" => {
                                        if let Some((_type, val)) =
                                            value.trim_start().rsplit_once(' ')
                                        {
                                            if let Ok(number) = val.parse::<u64>() {
                                                nwritten = number;
                                            }
                                        }
                                    }
                                    "nread" => {
                                        if let Some((_type, val)) =
                                            value.trim_start().rsplit_once(' ')
                                        {
                                            if let Ok(number) = val.parse::<u64>() {
                                                nread = number;
                                            }
                                        }
                                    }
                                    _ => continue,
                                }
                            }
                        }
                        if !ds_name.is_empty() {
                            let counter = IoCounters::new(ds_name.to_owned(), nread, nwritten);
                            //log::debug!("adding io counter for zfs {:?}", counter);
                            results.push(Ok(counter));
                        }
                    }
                }
            }
        });
    }
    Ok(results)
}
