use std::collections::HashMap;

use anyhow::{bail, ensure, Context, Result};

use jelly_mem_access::*;

use crate::json_as_map;
use crate::json_as_str;

const ACC_ADDRS: [(&'static str, usize); 5] = [
    ("INPUT_H", 0x10),
    ("INPUT_W", 0x18),
    ("FOLD_INPUT_CH", 0x20),
    ("LEAKY", 0x28),
    ("BIAS_EN", 0x30),
];
const CONV_ADDRS: [(&'static str, usize); 8] = [
    ("OUTPUT_CH", 0x10),
    ("INPUT_CH", 0x18),
    ("FOLD_OUTPUT_CH", 0x20),
    ("FOLD_INPUT_CH", 0x28),
    ("INPUT_H", 0x30),
    ("INPUT_W", 0x38),
    ("REAL_INPUT_H", 0x40),
    ("FOLD_WIN_AREA", 0x48),
];
const MAX_POOL_ADDRS: [(&'static str, usize); 6] = [
    ("OUTPUT_H", 0x10),
    ("OUTPUT_W", 0x18),
    ("INPUT_H", 0x20),
    ("INPUT_W", 0x28),
    ("INPUT_FOLD_CH", 0x30),
    ("STRIDE", 0x38),
];
const YOLO_ADDRS: [(&'static str, usize); 3] = [
    ("ACTIVATE_EN", 0x10),
    ("INPUT_H", 0x18),
    ("INPUT_W", 0x20),
];

fn get_addrs(name: &str) -> Result<HashMap<String, usize>> {
    let addr_iter = match name {
        "yolo_acc_top" => ACC_ADDRS.iter(),
        "yolo_conv_top" => CONV_ADDRS.iter(),
        "yolo_max_pool_top" => MAX_POOL_ADDRS.iter(),
        "yolo_upsamp_top" => [].iter(),
        "yolo_yolo_top" => YOLO_ADDRS.iter(),
        _ => bail!("This IP is not supported.")
    };
    Ok(addr_iter.map(|(k, v)| (k.to_string(), *v)).collect())
}


pub struct Yolo {
    uio_acc: UioAccessor<usize>,
    addrs: HashMap<String, usize>,
}

impl Yolo {
    pub fn new(hw_info: &serde_json::Value) -> Result<Self> {
        let hw_object = json_as_map!(hw_info);
        // let hw_params = json_as_map!(hw_object["params"]);
        let vendor = json_as_str!(hw_object["vendor"]);
        let library = json_as_str!(hw_object["library"]);
        let name = json_as_str!(hw_object["name"]);
        let uio_name = json_as_str!(hw_object["uio"]);
        ensure!(
            vendor == "xilinx.com" && library == "hls",
            "AxiDmaChannel::new(): This IP is not supported. ({})",
            name
        );
        let uio = match UioAccessor::<usize>::new_with_name(uio_name) {
            Ok(uio_acc) => uio_acc,
            Err(e) => {
                bail!("UioAccessor: {}", e)
            }
        };

        Ok(Yolo {
            uio_acc: uio,
            addrs: get_addrs(name)?,
        })
    }

    pub fn is_done(&self) -> bool {
        unsafe { self.uio_acc.read_mem32(0x00) & 2 == 2 }
    }

    pub fn is_idle(&self) -> bool {
        unsafe { self.uio_acc.read_mem32(0x00) & 4 == 4 }
    }

    pub fn is_ready(&self) -> bool {
        unsafe { self.uio_acc.read_mem32(0x00) & 1 != 1 }
    }

    pub fn start(&self) {
        let auto_restart = unsafe {
            self.uio_acc.read_mem32(0x00) & 0x80
        };
        unsafe {
            self.uio_acc.write_mem32(0x00, auto_restart | 0x01);
        }
    }

    pub fn set_auto_restart_enable(&self, en: bool) {
        let auto_restart = if en {
            0x80
        } else {
            0x00
        };
        unsafe {
            self.uio_acc.write_mem32(0x00, auto_restart);
        }
    }

    pub fn set(&self, name: &str, data: u32) {
        let addr = self.addrs[name];
        unsafe {
            self.uio_acc.write_mem32(addr, data);
        }
    }
    pub fn get(&self, name: &str) -> u32 {
        let addr = self.addrs[name];
        unsafe {
            self.uio_acc.read_mem32(addr)
        }
    }
}
