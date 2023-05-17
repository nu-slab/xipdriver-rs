#![allow(unused)]

use crate::{hwh_parser, mem};
use anyhow::{ensure, Result};

#[cfg(all(target_os = "linux", target_arch = "aarch64"))]
use jelly_mem_access::*;

const BIND_TO: [&str; 1] = ["xilinx.com:ip:axi_gpio:2.0"];

pub struct AxiGpio {
    pub hwh: hwh_parser::Ip,
    uio_acc: mem::UioAccessor<usize>,
    bitw: [u64; 2],
}

impl AxiGpio {
    pub fn new(hwh: &hwh_parser::Ip, uio_name: &str) -> Result<Self> {
        ensure!(
            BIND_TO.iter().any(|e| e == &hwh.vlnv),
            "AxiGpio::new(): This IP is not supported. ({})",
            hwh.vlnv
        );
        let uio = mem::new(uio_name)?;
        Ok(AxiGpio {
            hwh: hwh.clone(),
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
