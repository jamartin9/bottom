//! Disk stats for FreeBSD.

use rustc_hash::FxHashMap as HashMap;

use crate::collection::{
    DataCollector, disks::DiskHarvest, disks::IoData, disks::IoHarvest, disks::keep_disk_entry,
    error::CollectionResult,
};

pub fn get_io_usage(collector: &DataCollector) -> CollectionResult<IoHarvest> {
    // TODO: Should this (and other I/O collectors) fail fast? In general, should
    // collection ever fail fast?
    #[cfg_attr(not(feature = "zfs"), expect(unused_mut))]
    let mut io_harvest: HashMap<String, Option<IoData>> = collector
        .sys
        .disks
        .iter()
        .map(|disk| {
            (
                disk.name().to_string_lossy().to_string(),
                Some(IoData {
                    read_bytes: disk.usage().read_bytes,
                    write_bytes: disk.usage().written_bytes,
                }),
            )
        })
        .collect();

    #[cfg(feature = "zfs")]
    {
        use crate::collection::disks::zfs_io_counters;
        if let Ok(zfs_io) = zfs_io_counters::zfs_io_stats() {
            for io in zfs_io.into_iter() {
                let mount_point = io.device_name().to_string_lossy();
                io_harvest.insert(
                    mount_point.to_string(),
                    Some(IoData {
                        read_bytes: io.read_bytes(),
                        write_bytes: io.write_bytes(),
                    }),
                );
            }
        }
    }
    Ok(io_harvest)
}

pub(crate) fn get_disk_usage(collector: &DataCollector) -> anyhow::Result<Vec<DiskHarvest>> {
    let disks = &collector.sys.disks;
    let disk_filter = &collector.filters.disk_filter;
    let mount_filter = &collector.filters.mount_filter;

    // replace sysinfo name with zfs dataset name for zfs io-counters when mounts match
    #[cfg(feature = "zfs")]
    use crate::collection::disks::unix::FileSystem;
    #[cfg(feature = "zfs")]
    use std::{ffi::CStr, ptr::null_mut, str::FromStr};
    #[cfg(feature = "zfs")]
    let zfs_mounts: HashMap<String, String> = {
        let mut fs_infos: *mut libc::statfs = null_mut();
        let count = unsafe { libc::getmntinfo(&mut fs_infos, libc::MNT_WAIT) };
        if count < 1 {
            HashMap::default()
        } else {
            let fs_infos: &[libc::statfs] =
                unsafe { std::slice::from_raw_parts(fs_infos as _, count as _) };
            fs_infos
                .iter()
                .filter_map(|stat| {
                    if stat.f_mntfromname[0] == 0 || stat.f_mntonname[0] == 0 {
                        None
                    } else {
                        let fs_type = {
                            // SAFETY: Should be a non-null pointer.
                            let fs_type_str = unsafe {
                                CStr::from_ptr(stat.f_fstypename.as_ptr()).to_string_lossy()
                            };
                            FileSystem::from_str(&fs_type_str)
                                .unwrap_or(FileSystem::Other(fs_type_str.to_string()))
                        };
                        if fs_type == FileSystem::Zfs {
                            // SAFETY: Should be a non-null pointer.
                            let device = unsafe {
                                CStr::from_ptr(stat.f_mntfromname.as_ptr())
                                    .to_string_lossy()
                                    .to_string()
                            };
                            // SAFETY: Should be a non-null pointer.
                            let mount = unsafe {
                                CStr::from_ptr(stat.f_mntonname.as_ptr())
                                    .to_string_lossy()
                                    .to_string()
                            };
                            Some((mount, device))
                        } else {
                            None
                        }
                    }
                })
                .collect()
        }
    };

    Ok(disks
        .iter()
        .filter_map(|disk| {
            let mount_point = disk
                .mount_point()
                .as_os_str()
                .to_os_string()
                .into_string()
                .unwrap_or_else(|_| "Mount Unavailable".to_string());
            let name = {
                let name = disk.name();
                if name.is_empty() {
                    "No Name".to_string()
                } else {
                    #[cfg(feature = "zfs")]
                    if let Some(ds) = zfs_mounts.get(&mount_point) {
                        ds.to_owned()
                    } else {
                        name.to_os_string()
                            .into_string()
                            .unwrap_or_else(|_| "Name Unavailable".to_string())
                    }
                    #[cfg(not(feature = "zfs"))]
                    name.to_os_string()
                        .into_string()
                        .unwrap_or_else(|_| "Name Unavailable".to_string())
                }
            };

            if keep_disk_entry(&name, &mount_point, disk_filter, mount_filter) {
                let free_space = disk.available_space();
                let total_space = disk.total_space();
                let used_space = total_space - free_space;

                Some(DiskHarvest {
                    name,
                    mount_point,
                    free_space: Some(free_space),
                    used_space: Some(used_space),
                    total_space: Some(total_space),
                })
            } else {
                None
            }
        })
        .collect())
}
