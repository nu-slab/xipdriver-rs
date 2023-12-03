use anyhow::{bail, ensure, Context, Result};

use jelly_mem_access::*;

use crate::json_as_map;
use crate::json_as_str;


pub struct AxisSwitch {
    uio_acc: UioAccessor<usize>,
}

impl AxisSwitch {
    pub fn new(hw_info: &serde_json::Value) -> Result<Self> {
        let hw_object = json_as_map!(hw_info);
        let vendor = json_as_str!(hw_object["vendor"]);
        let library = json_as_str!(hw_object["library"]);
        let name = json_as_str!(hw_object["name"]);
        let uio_name = json_as_str!(hw_object["uio"]);
        ensure!(
            vendor == "xilinx.com" &&
            library == "ip" &&
            name == "axis_switch",
            "VideoFrameBufRead::new(): This IP is not supported. ({})",
            name
        );

        let uio = match UioAccessor::<usize>::new_with_name(uio_name) {
            Ok(uio_acc) => uio_acc,
            Err(e) => {
                bail!("UioAccessor: {}", e)
            }
        };

        Ok(AxisSwitch {
            uio_acc: uio,
        })
    }

    pub fn enable_mi_port(&self, mi_index: usize, si_port: u32) {
        let mi_port_addr = 0x40 + 4 * mi_index;

        unsafe {
            self.uio_acc.write_mem32(mi_port_addr, si_port);
        }
    }
    pub fn disable_mi_port(&self, mi_index: usize) {
        let mi_port_addr = 0x40 + 4 * mi_index;

        unsafe {
            self.uio_acc.write_mem32(mi_port_addr, 0x80000000);
        }
    }

    pub fn is_mi_port_enabled(&self, mi_index: usize, si_index: u32) -> bool {
        let mi_port_addr = 0x40 + 4 * mi_index;
        let mut reg_value = unsafe {
            self.uio_acc.read_mem32(mi_port_addr)
        };

        let enable = (reg_value >> 31) != 0;
        reg_value &= 0x0F;

        ((reg_value == si_index) && (!enable)) || ((reg_value & si_index) != 0 && (!enable))
    }

    pub fn is_mi_port_disabled(&self, mi_index: usize) -> bool {
        let mi_port_addr = 0x40 + 4 * mi_index;
        let reg_value = unsafe {
            self.uio_acc.read_mem32(mi_port_addr)
        };

        (reg_value >> 31) != 0
    }

    pub fn disable_all_mi_ports(&self) {
        for mi_index in 0..16 {
            let mi_port_addr = 0x40 + 4 * mi_index;

            unsafe {
                self.uio_acc.write_mem32(mi_port_addr, 0x80000000);
            }
        }
    }

    pub fn reg_update_enable(&self) {
        unsafe {
            let reg_value = self.uio_acc.read_mem32(0x00);
            self.uio_acc.write_mem32(0x00, reg_value | 0x02);
        }
    }

    pub fn reg_update_disable(&self) {
        unsafe {
            let reg_value = self.uio_acc.read_mem32(0x00);
            self.uio_acc.write_mem32(0x00, reg_value & (!0x02));
        }
    }
}
