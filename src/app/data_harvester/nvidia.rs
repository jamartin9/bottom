use nvml_wrapper::enum_wrappers::device::TemperatureSensor;
//use nvml_wrapper::struct_wrappers::device::ProcessUtilizationSample;
use hashbrown::HashMap;
use nvml_wrapper::{error::NvmlError, Nvml};
use once_cell::sync::Lazy;

use crate::app::Filter;

use crate::data_harvester::memory::MemHarvest;
use crate::data_harvester::temperature::{
    convert_celsius_to_fahrenheit, convert_celsius_to_kelvin, is_temp_filtered, TempHarvest,
    TemperatureType,
};

pub static NVML_DATA: Lazy<Result<Nvml, NvmlError>> = Lazy::new(Nvml::init);

//pub type GpuLoadAvgHarvest = [u32; 3];

pub enum GpuUtilType {
    Avg,
    Gpu(usize),
}
pub struct GpuUtil {
    pub data_type: GpuUtilType,
    pub gpu_usage: u32,
}
/*
pub struct GpuData {
    pub name: String,
    pub memory: Option<MemHarvest>,
    pub temperature: Option<TempHarvest>,
    pub usage: Option<GpuUtil>,
    pub procs: Option<HashMap<u32, (u32, u32)>>,
    #[cfg(feature = "battery")]
    pub battery: Option<BatteryHarvest>,
}
*/

/// Returns the Gpu data of NVIDIA cards.
#[inline]
pub fn get_nvidia_vecs(
    temp_type: &TemperatureType, filter: &Option<Filter>, use_temp: bool, use_mem: bool,
    use_proc: bool, use_cpu: bool, use_battery: bool,
) -> Option<(
    Option<Vec<TempHarvest>>,
    Option<Vec<(String, MemHarvest)>>,
    Option<Vec<GpuUtil>>,
    Option<Vec<HashMap<u32, (u32, u32)>>>,
    Option<Vec<u32>>,
)> {
    if let Ok(nvml) = &*NVML_DATA {
        if let Ok(num_gpu) = nvml.device_count() {
            let mut temp_vec = Vec::with_capacity(num_gpu as usize);
            let mut mem_vec = Vec::with_capacity(num_gpu as usize);
            let mut util_vec = Vec::with_capacity(num_gpu as usize);
            let mut proc_vec = Vec::with_capacity(num_gpu as usize);
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
                                    temp_vec.push(TempHarvest { name, temperature });
                                }
                            }
                        }
                        if use_cpu {
                            if let Ok(util) = device.utilization_rates() {
                                util_vec.push(GpuUtil {
                                    gpu_usage: util.gpu,
                                    data_type: GpuUtilType::Gpu(i as usize),
                                });
                            }
                        }
                        if use_proc {
                            if let Ok(gpu_procs) = device.process_utilization_stats(None) {
                                let mut procs = HashMap::with_capacity(gpu_procs.len());
                                for proc in gpu_procs {
                                    let pid = proc.pid;
                                    let gpu_mem = proc.mem_util;
                                    let gpu_util = proc.sm_util + proc.enc_util + proc.dec_util;
                                    log::debug!("Inserting proc for {:#?}", pid);
                                    procs.insert(pid, (gpu_mem, gpu_util));
                                }
                                proc_vec.push(procs);
                            }
                        }

                        if use_battery {
                            if let Ok(power) = device.power_usage() {
                                power_vec.push(power);
                            }
                        }
                    }
                }
            }
            Some((
                if temp_vec.is_empty() {
                    None
                } else {
                    Some(temp_vec)
                },
                if mem_vec.is_empty() {
                    None
                } else {
                    Some(mem_vec)
                },
                if util_vec.is_empty() {
                    None
                } else {
                    Some(util_vec)
                },
                if proc_vec.is_empty() {
                    None
                } else {
                    Some(proc_vec)
                },
                if power_vec.is_empty() {
                    None
                } else {
                    Some(power_vec)
                },
            ))
        } else {
            None
        }
    } else {
        None
    }
}
