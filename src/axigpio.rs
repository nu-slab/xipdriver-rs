#![allow(unused)]

use anyhow::{ensure, Result, Context, bail};

#[cfg(all(target_os = "linux", target_arch = "aarch64"))]
use jelly_mem_access::*;

const BIND_TO: [&str; 1] = ["xilinx.com:ip:axi_gpio:2.0"];

pub struct AxiGpio {
    uio_acc: UioAccessor<usize>,
    bitw: [u64; 2],
}

impl AxiGpio {
    pub fn new(hw_info: &serde_json::Value) -> Result<Self> {
        let hw_object = hw_info.as_object().context("hw_object is not an object type")?;
        let hw_params = hw_object["params"].as_object().context("hw_params is not an object type")?;
        let vendor = hw_object["vendor"].as_str().context("vendor is not string")?;
        let library = hw_object["library"].as_str().context("library is not string")?;
        let name = hw_object["name"].as_str().context("name is not string")?;
        let uio_name = hw_object["uio"].as_str().context("uio_name is not string")?;
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
