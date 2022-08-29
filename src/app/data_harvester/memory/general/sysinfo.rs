//! Data collection for memory via sysinfo.

use crate::data_harvester::memory::MemHarvest;
use sysinfo::{System, SystemExt};

pub async fn get_mem_data(
    sys: &System, actually_get: bool,
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
            get_ram_data(sys),
            get_swap_data(sys),
            get_arc_data(),
            get_gpu_data()
        )
    }
}

pub async fn get_ram_data(sys: &System) -> crate::utils::error::Result<Option<MemHarvest>> {
    let (mem_total_in_kib, mem_used_in_kib) = (sys.total_memory(), sys.used_memory());

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

pub async fn get_swap_data(sys: &System) -> crate::utils::error::Result<Option<MemHarvest>> {
    let (mem_total_in_kib, mem_used_in_kib) = (sys.total_swap(), sys.used_swap());

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
