//! Data collection for memory via sysinfo.

use crate::data_harvester::memory::MemHarvest;
use sysinfo::{System, SystemExt};

pub async fn get_mem_data(
    sys: &System, actually_get: bool,
) -> (
    crate::utils::error::Result<Option<MemHarvest>>,
    crate::utils::error::Result<Option<MemHarvest>>,
    crate::utils::error::Result<Option<MemHarvest>>,
    crate::utils::error::Result<Option<MemHarvest>>,
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

pub async fn get_gpu_data() -> crate::utils::error::Result<Option<MemHarvest>> {
    #[cfg(not(feature = "nvidia"))]
    let (mem_total_in_kib, mem_used_in_kib) = (0, 0);

    #[cfg(feature = "nvidia")]
    let (mem_total_in_kib, mem_used_in_kib) = {
        use crate::data_harvester::nvidia::NVML_DATA;
        let mut mem_total = 0;
        let mut mem_used = 0;
        if let Ok(nvml) = &*NVML_DATA {
            if let Ok(ngpu) = nvml.device_count() {
                for i in 0..ngpu {
                    if let Ok(device) = nvml.device_by_index(i) {
                        if let Ok(mem) = device.memory_info() {
                            // add all devices memory in bytes
                            mem_total += mem.total;
                            mem_used += mem.used;
                        }
                    }
                }
                (mem_total / 1024, mem_used / 1024)
            } else {
                (0, 0)
            }
        } else {
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
