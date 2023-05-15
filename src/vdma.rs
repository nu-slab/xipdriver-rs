#![allow(unused)]

use crate::{hwh_parser, mem};
use anyhow::{ensure, Result};
use jelly_mem_access::*;

const BIND_TO: [&str; 2] = ["xilinx.com:ip:axi_vdma:6.2", "xilinx.com:ip:axi_vdma:6.3"];
const MM2S_DMACR: usize = 0x00;
const MM2S_DMASR: usize = 0x04;
const MM2S_FRMSTORE: usize = 0x18;
const PARK_PTR_REG: usize = 0x28;
const VDMA_VERSION: usize = 0x2C;
const S2MM_DMACR: usize = 0x30;
const S2MM_DMASR: usize = 0x34;
const S2MM_FRMSTORE: usize = 0x48;
const MM2S_VSIZE: usize = 0x50;
const MM2S_HSIZE: usize = 0x54;
const MM2S_FRMDLY_STRIDE: usize = 0x58;
const MM2S_START_ADDRESS1: usize = 0x5C;
const MM2S_START_ADDRESS2: usize = 0x60;
const MM2S_START_ADDRESS3: usize = 0x64;
const MM2S_START_ADDRESS4: usize = 0x68;
const MM2S_START_ADDRESS5: usize = 0x6C;
const MM2S_START_ADDRESS6: usize = 0x70;
const MM2S_START_ADDRESS7: usize = 0x74;
const MM2S_START_ADDRESS8: usize = 0x78;
const MM2S_START_ADDRESS9: usize = 0x7C;
const MM2S_START_ADDRESS10: usize = 0x80;
const MM2S_START_ADDRESS11: usize = 0x84;
const MM2S_START_ADDRESS12: usize = 0x88;
const MM2S_START_ADDRESS13: usize = 0x8C;
const MM2S_START_ADDRESS14: usize = 0x90;
const MM2S_START_ADDRESS15: usize = 0x94;
const MM2S_START_ADDRESS16: usize = 0x98;
const S2MM_VSIZE: usize = 0xA0;
const S2MM_HSIZE: usize = 0xA4;
const S2MM_FRMDLY_STRIDE: usize = 0xA8;
const S2MM_START_ADDRESS1: usize = 0xAC;

pub struct AxiVdma {
    pub hwh: hwh_parser::Ip,
    uio_acc: mem::UioAccessor<usize>,
}

impl AxiVdma {
    pub fn new(hwh: &hwh_parser::Ip) -> Result<Self> {
        ensure!(
            BIND_TO.iter().any(|e| e == &hwh.vlnv),
            "AxiVdma::new(): This IP is not supported. ({})",
            hwh.vlnv
        );
        let uio = mem::new("axi_vdma").unwrap();
        Ok(AxiVdma {
            hwh: hwh.clone(),
            uio_acc: uio//.subclone(hwh.phys_addr, hwh.addr_range),
        })
    }

    pub fn is_running(&self) -> bool {
        unsafe {
            self.uio_acc.read_reg(MM2S_DMASR) & 1 == 1
        }
    }

    fn allocate(&self) -> Result<()> {
        let offset = 0;
        let mem_phys_addr = 0;
        ensure!(offset > 15, "allocate err");
        unsafe {
            self.uio_acc.write_reg(MM2S_START_ADDRESS1 + 4 * offset, mem_phys_addr)
        }
        Ok(())
    }

    pub fn write_mode(&self) {
        let width = 1280;
        let height = 720;
        let bytes_per_pix = 3;
        let stride = 1;
        unsafe {
            self.uio_acc.write_reg(MM2S_HSIZE, width * bytes_per_pix);
        }
        let mut reg = unsafe {
            self.uio_acc.read_reg(MM2S_FRMDLY_STRIDE)
        };
        reg &= 0xF << 24;
        reg |= stride;
        unsafe {
            self.uio_acc.write_reg(MM2S_FRMDLY_STRIDE, reg);
        }
    }

    pub fn reload(&self) {
        let height = 720;
        unsafe {
            self.uio_acc.write_reg(MM2S_HSIZE, height);
        }
    }

    pub fn stop(&self) {
        unsafe {
            self.uio_acc.write_reg(MM2S_DMACR, 0x00011080);
        }
        while self.is_running() {}
    }

    pub fn reset(&self) {
        self.stop();
        unsafe {
            self.uio_acc.write_reg(MM2S_DMACR, 0x00011084);
            while self.uio_acc.read_reg(MM2S_DMACR) & 4 == 4 {}
        }
    }

    pub fn write_frame(&self) {
        unimplemented!();
    }

    fn get_desired_frame(&self) -> usize {
        unsafe {
            self.uio_acc.read_reg(PARK_PTR_REG) & 0x1F
        }
    }

    fn set_desired_frame(&self, frame_num: usize) {
        let mut reg = unsafe {
            self.uio_acc.read_reg(PARK_PTR_REG)
        };
        reg &=  !0x1F;
        reg |=  frame_num;
        unsafe {
            self.uio_acc.write_reg(PARK_PTR_REG, reg);
        }
    }
}
