#![allow(unused)]

use anyhow::{ensure, Result, Context, bail};

#[cfg(all(target_os = "linux", target_arch = "aarch64"))]
use jelly_mem_access::*;

use crate::json_as_map;
use crate::json_as_str;

pub struct AxiGpio {
    uio_acc: UioAccessor<usize>,
    bitw: [u64; 2],
}

impl AxiGpio {
    pub fn new(hw_info: &serde_json::Value) -> Result<Self> {
        let hw_object = json_as_map!(hw_info);
        // let hw_params = json_as_map!(hw_object["params"]);
        let vendor = json_as_str!(hw_object["vendor"]);
        let library = json_as_str!(hw_object["library"]);
        let name = json_as_str!(hw_object["name"]);
        let uio_name = json_as_str!(hw_object["uio"]);
        ensure!(
            vendor == "xilinx.com" &&
            library == "ip" &&
            name == "axi_gpio",
            "VideoFrameBufRead::new(): This IP is not supported. ({})",
            name
        );
        let uio = match UioAccessor::<usize>::new_with_name(uio_name) {
            Ok(uio_acc) => {
                uio_acc
            },
            Err(e) => {
                bail!("UioAccessor: {}", e)
            }
        };
        Ok(AxiGpio {
            uio_acc: uio,
            bitw: [32, 32],
        })
    }

    pub fn read_data(&self, channel: usize) -> Result<u32> {
        ensure!(channel == 1 || channel == 2, "channel bust be 1 or 2");
        let offset = if channel == 1 { 0x00 } else { 0x08 };
        Ok(unsafe {
            self.uio_acc.read_mem32(offset)
        })
    }
    pub fn write_data(&self, channel: usize, data:u32) -> Result<()> {
        ensure!(channel == 1 || channel == 2, "channel bust be 1 or 2");
        let offset = if channel == 1 { 0x00 } else { 0x08 };
        unsafe {
            self.uio_acc.write_mem32(offset, data);
        }
        Ok(())
    }
    pub fn read_tri(&self, channel: usize) -> Result<u32> {
        ensure!(channel == 1 || channel == 2, "channel bust be 1 or 2");
        let offset = if channel == 1 { 0x04 } else { 0x0C };
        Ok(unsafe {
            self.uio_acc.read_mem32(offset)
        })
    }
    pub fn write_tri(&self, channel: usize, data:u32) -> Result<()> {
        ensure!(channel == 1 || channel == 2, "channel bust be 1 or 2");
        let offset = if channel == 1 { 0x04 } else { 0x0C };
        unsafe {
            self.uio_acc.write_mem32(offset, data);
        }
        Ok(())
    }

}
