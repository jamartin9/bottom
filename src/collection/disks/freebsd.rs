//! Disk stats for FreeBSD.

use rustc_hash::FxHashMap as HashMap;

use super::IoHarvest;

use crate::collection::{DataCollector, disks::IoData, error::CollectionResult};

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
                    mount_point.to_string(), // needs patch to sysinfo disks to not use "root" or mount_points. // let name = if &fs_type[..] == b"zfs"{ OsString::from(c_buf_to_utf8_str(&fs_info.f_mntfromname).unwrap_or("")) }
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
