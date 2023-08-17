use hashbrown::HashMap;
use nvml_wrapper::enum_wrappers::device::TemperatureSensor;
use nvml_wrapper::enums::device::UsedGpuMemory;
use nvml_wrapper::{error::NvmlError, Nvml};
use once_cell::sync::Lazy;

use crate::app::Filter;

use crate::data_harvester::cpu::{CpuData, CpuDataType};
use crate::data_harvester::memory::MemHarvest;
use crate::data_harvester::temperature::{
    convert_celsius_to_fahrenheit, convert_celsius_to_kelvin, is_temp_filtered, TempHarvest,
    TemperatureType,
};

pub static NVML_DATA: Lazy<Result<Nvml, NvmlError>> = Lazy::new(Nvml::init);

pub struct GpusData {
    pub memory: Option<Vec<(String, MemHarvest)>>,
    pub temperature: Option<Vec<TempHarvest>>,
    pub usage: Option<Vec<CpuData>>,
    pub procs: Option<Vec<HashMap<u32, (u64, u32)>>>,
    #[cfg(feature = "battery")]
    pub battery: Option<Vec<(String, u32)>>,
}

/// Returns the Gpu data of NVIDIA cards.
#[inline]
pub fn get_nvidia_vecs(
    temp_type: &TemperatureType, filter: &Option<Filter>, use_temp: bool, use_mem: bool,
    use_proc: bool, use_cpu: bool, use_battery: bool,
) -> Option<GpusData> {
    if let Ok(nvml) = &*NVML_DATA {
        if let Ok(num_gpu) = nvml.device_count() {
            let mut temp_vec = Vec::with_capacity(num_gpu as usize);
            let mut mem_vec = Vec::with_capacity(num_gpu as usize);
            let mut util_vec = Vec::with_capacity(num_gpu as usize);
            let mut proc_vec = Vec::with_capacity(num_gpu as usize);
            #[cfg(feature = "battery")]
            let mut power_vec = Vec::with_capacity(num_gpu as usize);

            for i in 0..num_gpu {
                if let Ok(device) = nvml.device_by_index(i) {
                    if let Ok(name) = device.name() {
                        if use_mem {
                            if let Ok(mem) = device.memory_info() {
                                mem_vec.push((
                                    name.clone(),
                                    MemHarvest {
                                        total_bytes: mem.total,
                                        used_bytes: mem.used,
                                        use_percent: if mem.total == 0 {
                                            None
                                        } else {
                                            Some(mem.used as f64 / mem.total as f64 * 100.0)
                                        },
                                    },
                                ));
                            }
                        }
                        if use_temp {
                            if let Ok(temperature) = device.temperature(TemperatureSensor::Gpu) {
                                if is_temp_filtered(filter, &name) {
                                    let temperature = temperature as f32;
                                    let temperature = match temp_type {
                                        TemperatureType::Celsius => temperature,
                                        TemperatureType::Kelvin => {
                                            convert_celsius_to_kelvin(temperature)
                                        }
                                        TemperatureType::Fahrenheit => {
                                            convert_celsius_to_fahrenheit(temperature)
                                        }
                                    };
                                    temp_vec.push(TempHarvest {
                                        name: name.clone(),
                                        temperature,
                                    });
                                }
                            }
                        }
                        if use_cpu {
                            if let Ok(util) = device.utilization_rates() {
                                util_vec.push(CpuData {
                                    cpu_usage: util.gpu as f64,
                                    data_type: CpuDataType::Gpu(i as usize),
                                });
                            }
                        }
                        if use_proc {
                            let mut procs = HashMap::new();
                            if let Ok(gpu_procs) = device.process_utilization_stats(None) {
                                for proc in gpu_procs {
                                    let pid = proc.pid;
                                    let gpu_util = proc.sm_util + proc.enc_util + proc.dec_util;
                                    procs.insert(pid, (0, gpu_util));
                                }
                            }
                            if let Ok(compute_procs) = device.running_compute_processes() {
                                for proc in compute_procs {
                                    let pid = proc.pid;
                                    let gpu_mem = match proc.used_gpu_memory {
                                        UsedGpuMemory::Used(val) => val,
                                        UsedGpuMemory::Unavailable => 0,
                                    };
                                    if let Some(prev) = procs.get(&pid) {
                                        procs.insert(pid, (gpu_mem, prev.1));
                                    } else {
                                        procs.insert(pid, (gpu_mem, 0));
                                    }
                                }
                            }
                            if let Ok(graphics_procs) = device.running_graphics_processes() {
                                for proc in graphics_procs {
                                    let pid = proc.pid;
                                    let gpu_mem = match proc.used_gpu_memory {
                                        UsedGpuMemory::Used(val) => val,
                                        UsedGpuMemory::Unavailable => 0,
                                    };
                                    if let Some(prev) = procs.get(&pid) {
                                        procs.insert(pid, (gpu_mem, prev.1));
                                    } else {
                                        procs.insert(pid, (gpu_mem, 0));
                                    }
                                }
                            }
                            if !procs.is_empty() {
                                proc_vec.push(procs);
                            }
                        }

                        #[cfg(feature = "battery")]
                        if use_battery {
                            if let Ok(power) = device.power_usage() {
                                power_vec.push((name, power));
                            }
                        }
                    }
                }
            }
            Some(GpusData {
                memory: if !mem_vec.is_empty() {
                    Some(mem_vec)
                } else {
                    None
                },
                temperature: if !temp_vec.is_empty() {
                    Some(temp_vec)
                } else {
                    None
                },
                usage: if !util_vec.is_empty() {
                    Some(util_vec)
                } else {
                    None
                },
                procs: if !proc_vec.is_empty() {
                    Some(proc_vec)
                } else {
                    None
                },
                #[cfg(feature = "battery")]
                battery: if !power_vec.is_empty() {
                    Some(power_vec)
                } else {
                    None
                },
            })
        } else {
            None
        }
    } else {
        None
    }
}
