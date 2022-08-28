//! Data collection for memory via heim.

use crate::data_harvester::memory::MemHarvest;

pub async fn get_mem_data(
    actually_get: bool,
) -> (
    crate::utils::error::Result<Option<MemHarvest>>,
    crate::utils::error::Result<Option<MemHarvest>>,
    crate::utils::error::Result<Option<MemHarvest>>,
    crate::utils::error::Result<Option<Vec<(String, MemHarvest)>>>,
) {
    use futures::join;

    if !actually_get {
        (Ok(None), Ok(None), Ok(None), Ok(None))
    } else {
        join!(
            get_ram_data(),
            get_swap_data(),
            get_arc_data(),
            get_gpu_data()
        )
    }
}

pub async fn get_ram_data() -> crate::utils::error::Result<Option<MemHarvest>> {
    let (mem_total_in_kib, mem_used_in_kib) = {
        #[cfg(target_os = "linux")]
        {
            use smol::fs::read_to_string;
            let meminfo = read_to_string("/proc/meminfo").await?;

            // All values are in KiB by default.
            let mut mem_total = 0;
            let mut cached = 0;
            let mut s_reclaimable = 0;
            let mut shmem = 0;
            let mut buffers = 0;
            let mut mem_free = 0;

            let mut keys_read: u8 = 0;
            const TOTAL_KEYS_NEEDED: u8 = 6;

            for line in meminfo.lines() {
                if let Some((label, value)) = line.split_once(':') {
                    let to_write = match label {
                        "MemTotal" => &mut mem_total,
                        "MemFree" => &mut mem_free,
                        "Buffers" => &mut buffers,
                        "Cached" => &mut cached,
                        "Shmem" => &mut shmem,
                        "SReclaimable" => &mut s_reclaimable,
                        _ => {
                            continue;
                        }
                    };

                    if let Some((number, _unit)) = value.trim_start().split_once(' ') {
                        // Parse the value, remember it's in KiB!
                        if let Ok(number) = number.parse::<u64>() {
                            *to_write = number;

                            // We only need a few keys, so we can bail early.
                            keys_read += 1;
                            if keys_read == TOTAL_KEYS_NEEDED {
                                break;
                            }
                        }
                    }
                }
            }

            // Let's preface this by saying that memory usage calculations are... not straightforward.
            // There are conflicting implementations everywhere.
            //
            // Now that we've added this preface (mainly for future reference), the current implementation below for usage
            // is based on htop's calculation formula. See
            // https://github.com/htop-dev/htop/blob/976c6123f41492aaf613b9d172eef1842fb7b0a3/linux/LinuxProcessList.c#L1584
            // for implementation details as of writing.
            //
            // Another implementation, commonly used in other things, is to skip the shmem part of the calculation,
            // which matches gopsutil and stuff like free.

            let total = mem_total;
            let cached_mem = cached + s_reclaimable - shmem;
            let used_diff = mem_free + cached_mem + buffers;
            let used = if total >= used_diff {
                total - used_diff
            } else {
                total - mem_free
            };

            (total, used)
        }
        #[cfg(target_os = "macos")]
        {
            let memory = heim::memory::memory().await?;

            use heim::memory::os::macos::MemoryExt;
            use heim::units::information::kibibyte;
            (
                memory.total().get::<kibibyte>(),
                memory.active().get::<kibibyte>() + memory.wire().get::<kibibyte>(),
            )
        }
        #[cfg(target_os = "windows")]
        {
            let memory = heim::memory::memory().await?;

            use heim::units::information::kibibyte;
            let mem_total_in_kib = memory.total().get::<kibibyte>();
            (
                mem_total_in_kib,
                mem_total_in_kib - memory.available().get::<kibibyte>(),
            )
        }
        #[cfg(target_os = "freebsd")]
        {
            let mut s = System::new();
            s.refresh_memory();
            (s.total_memory(), s.used_memory())
        }
    };

    Ok(Some(MemHarvest {
        mem_total_in_kib,
        mem_used_in_kib,
        use_percent: if mem_total_in_kib == 0 {
            None
        } else {
            Some(mem_used_in_kib as f64 / mem_total_in_kib as f64 * 100.0)
        },
    }))
}

pub async fn get_swap_data() -> crate::utils::error::Result<Option<MemHarvest>> {
    #[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
    let memory = heim::memory::swap().await?;
    #[cfg(target_os = "freebsd")]
    let mut memory = System::new();

    let (mem_total_in_kib, mem_used_in_kib) = {
        #[cfg(target_os = "linux")]
        {
            // Similar story to above - heim parses this information incorrectly as far as I can tell, so kilobytes = kibibytes here.
            use heim::units::information::kilobyte;
            (
                memory.total().get::<kilobyte>(),
                memory.used().get::<kilobyte>(),
            )
        }
        #[cfg(any(target_os = "windows", target_os = "macos"))]
        {
            use heim::units::information::kibibyte;
            (
                memory.total().get::<kibibyte>(),
                memory.used().get::<kibibyte>(),
            )
        }
        #[cfg(target_os = "freebsd")]
        {
            memory.refresh_memory();
            (memory.total_swap(), memory.used_swap())
        }
    };

    Ok(Some(MemHarvest {
        mem_total_in_kib,
        mem_used_in_kib,
        use_percent: if mem_total_in_kib == 0 {
            None
        } else {
            Some(mem_used_in_kib as f64 / mem_total_in_kib as f64 * 100.0)
        },
    }))
}

pub async fn get_arc_data() -> crate::utils::error::Result<Option<MemHarvest>> {
    #[cfg(not(feature = "zfs"))]
    let (mem_total_in_kib, mem_used_in_kib) = (0, 0);

    #[cfg(feature = "zfs")]
    let (mem_total_in_kib, mem_used_in_kib) = {
        #[cfg(target_os = "linux")]
        {
            let mut mem_arc = 0;
            let mut mem_total = 0;
            let mut zfs_keys_read: u8 = 0;
            const ZFS_KEYS_NEEDED: u8 = 2;
            use smol::fs::read_to_string;
            let arcinfo = read_to_string("/proc/spl/kstat/zfs/arcstats").await?;
            for line in arcinfo.lines() {
                if let Some((label, value)) = line.split_once(' ') {
                    let to_write = match label {
                        "size" => &mut mem_arc,
                        "memory_all_bytes" => &mut mem_total,
                        _ => {
                            continue;
                        }
                    };

                    if let Some((_type, number)) = value.trim_start().rsplit_once(' ') {
                        // Parse the value, remember it's in bytes!
                        if let Ok(number) = number.parse::<u64>() {
                            *to_write = number;
                            // We only need a few keys, so we can bail early.
                            zfs_keys_read += 1;
                            if zfs_keys_read == ZFS_KEYS_NEEDED {
                                break;
                            }
                        }
                    }
                }
            }
            (mem_total / 1024, mem_arc / 1024)
        }

        #[cfg(target_os = "freebsd")]
        {
            use sysctl::Sysctl;
            if let (Ok(mem_arc_value), Ok(mem_sys_value)) = (
                sysctl::Ctl::new("kstat.zfs.misc.arcstats.size"),
                sysctl::Ctl::new("hw.physmem"),
            ) {
                if let (Ok(sysctl::CtlValue::U64(arc)), Ok(sysctl::CtlValue::Ulong(mem))) =
                    (mem_arc_value.value(), mem_sys_value.value())
                {
                    (mem / 1024, arc / 1024)
                } else {
                    (0, 0)
                }
            } else {
                (0, 0)
            }
        }
        #[cfg(target_os = "macos")]
        {
            (0, 0)
        }
        #[cfg(target_os = "windows")]
        {
            (0, 0)
        }
    };

    Ok(Some(MemHarvest {
        mem_total_in_kib,
        mem_used_in_kib,
        use_percent: if mem_total_in_kib == 0 {
            None
        } else {
            Some(mem_used_in_kib as f64 / mem_total_in_kib as f64 * 100.0)
        },
    }))
}

pub async fn get_gpu_data() -> crate::utils::error::Result<Option<Vec<(String, MemHarvest)>>> {
    #[cfg(not(feature = "nvidia"))]
    {
        Ok(None)
    }

    #[cfg(feature = "nvidia")]
    {
        use crate::data_harvester::nvidia::NVML_DATA;
        if let Ok(nvml) = &*NVML_DATA {
            if let Ok(ngpu) = nvml.device_count() {
                let mut results = Vec::with_capacity(ngpu as usize);
                for i in 0..ngpu {
                    if let Ok(device) = nvml.device_by_index(i) {
                        if let (Ok(name), Ok(mem)) = (device.name(), device.memory_info()) {
                            // add device memory in bytes
                            let mem_total_in_kib = mem.total / 1024;
                            let mem_used_in_kib = mem.used / 1024;
                            // TODO REMOVE
                            let name2 = name.clone();
                            let mem_total_in_kib2 = mem_total_in_kib * 2;
                            let mem_used_in_kib2 = mem_used_in_kib + 1024 * 2;
                            let name3 = name.clone();
                            let mem_total_in_kib3 = mem_total_in_kib * 3;
                            let mem_used_in_kib3 = mem_used_in_kib + 1024 * 3;
                            let name4 = name.clone();
                            let mem_total_in_kib4 = mem_total_in_kib * 4;
                            let mem_used_in_kib4 = mem_used_in_kib + 1024 * 4;
                            let name5 = name.clone();
                            let mem_total_in_kib5 = mem_total_in_kib * 5;
                            let mem_used_in_kib5 = mem_used_in_kib + 1024 * 5;
                            let name6 = name.clone();
                            let mem_total_in_kib6 = mem_total_in_kib * 6;
                            let mem_used_in_kib6 = mem_used_in_kib + 1024 * 6;
                            let name7 = name.clone();
                            let mem_total_in_kib7 = mem_total_in_kib * 7;
                            let mem_used_in_kib7 = mem_used_in_kib + 1024 * 7;
                            let name8 = name.clone();
                            let mem_total_in_kib8 = mem_total_in_kib * 8;
                            let mem_used_in_kib8 = mem_used_in_kib + 1024 * 8;
                            // TODO REMOVE
                            results.push((
                                name,
                                MemHarvest {
                                    mem_total_in_kib,
                                    mem_used_in_kib,
                                    use_percent: if mem_total_in_kib == 0 {
                                        None
                                    } else {
                                        Some(
                                            mem_used_in_kib as f64 / mem_total_in_kib as f64
                                                * 100.0,
                                        )
                                    },
                                },
                            ));
                            // TODO REMOVE
                            results.push((
                                name2,
                                MemHarvest {
                                    mem_total_in_kib: mem_total_in_kib2,
                                    mem_used_in_kib: mem_used_in_kib2,
                                    use_percent: if mem_total_in_kib2 == 0 {
                                        None
                                    } else {
                                        Some(
                                            mem_used_in_kib2 as f64 / mem_total_in_kib2 as f64
                                                * 100.0,
                                        )
                                    },
                                },
                            ));
                            results.push((
                                name3,
                                MemHarvest {
                                    mem_total_in_kib: mem_total_in_kib3,
                                    mem_used_in_kib: mem_used_in_kib3,
                                    use_percent: if mem_total_in_kib3 == 0 {
                                        None
                                    } else {
                                        Some(
                                            mem_used_in_kib3 as f64 / mem_total_in_kib3 as f64
                                                * 100.0,
                                        )
                                    },
                                },
                            ));
                            results.push((
                                name4,
                                MemHarvest {
                                    mem_total_in_kib: mem_total_in_kib4,
                                    mem_used_in_kib: mem_used_in_kib4,
                                    use_percent: if mem_total_in_kib4 == 0 {
                                        None
                                    } else {
                                        Some(
                                            mem_used_in_kib4 as f64 / mem_total_in_kib4 as f64
                                                * 100.0,
                                        )
                                    },
                                },
                            ));
                            results.push((
                                name5,
                                MemHarvest {
                                    mem_total_in_kib: mem_total_in_kib5,
                                    mem_used_in_kib: mem_used_in_kib5,
                                    use_percent: if mem_total_in_kib5 == 0 {
                                        None
                                    } else {
                                        Some(
                                            mem_used_in_kib5 as f64 / mem_total_in_kib5 as f64
                                                * 100.0,
                                        )
                                    },
                                },
                            ));
                            results.push((
                                name6,
                                MemHarvest {
                                    mem_total_in_kib: mem_total_in_kib6,
                                    mem_used_in_kib: mem_used_in_kib6,
                                    use_percent: if mem_total_in_kib6 == 0 {
                                        None
                                    } else {
                                        Some(
                                            mem_used_in_kib6 as f64 / mem_total_in_kib6 as f64
                                                * 100.0,
                                        )
                                    },
                                },
                            ));
                            results.push((
                                name7,
                                MemHarvest {
                                    mem_total_in_kib: mem_total_in_kib7,
                                    mem_used_in_kib: mem_used_in_kib7,
                                    use_percent: if mem_total_in_kib7 == 0 {
                                        None
                                    } else {
                                        Some(
                                            mem_used_in_kib7 as f64 / mem_total_in_kib7 as f64
                                                * 100.0,
                                        )
                                    },
                                },
                            ));
                            results.push((
                                name8,
                                MemHarvest {
                                    mem_total_in_kib: mem_total_in_kib8,
                                    mem_used_in_kib: mem_used_in_kib8,
                                    use_percent: if mem_total_in_kib8 == 0 {
                                        None
                                    } else {
                                        Some(
                                            mem_used_in_kib8 as f64 / mem_total_in_kib8 as f64
                                                * 100.0,
                                        )
                                    },
                                },
                            ));
                            // TODO REMOVE
                        }
                    }
                }
                Ok(Some(results))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }
}
