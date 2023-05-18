#![allow(unused)]

use crate::{hwh_parser, mem};
use anyhow::{ensure, Result};

#[cfg(all(target_os = "linux", target_arch = "aarch64"))]
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
    udmabuf_acc: mem::UdmabufAccessor<usize>,
    image_width: u32,
    image_height: u32,
    image_bytes_per_pix: u32,
    image_stride: u32,
}

impl AxiVdma {
    pub fn new(hwh: &hwh_parser::Ip, uio_name: &str, udmabuf_name: &str) -> Result<Self> {
        ensure!(
            BIND_TO.iter().any(|e| e == &hwh.vlnv),
            "AxiVdma::new(): This IP is not supported. ({})",
            hwh.vlnv
        );
        let uio = mem::new(uio_name)?;
        let udmabuf = UdmabufAccessor::new(udmabuf_name, false).unwrap();
        Ok(AxiVdma {
            hwh: hwh.clone(),
            uio_acc: uio,
            udmabuf_acc: udmabuf,
            image_height: 1280,
            image_width: 720,
            image_bytes_per_pix: 4,
            image_stride: 4,
        })
    }

    pub fn is_running(&self) -> bool {
        unsafe {
            self.uio_acc.read_mem32(MM2S_DMASR) & 1 == 0
        }
    }

    fn allocate(&self) -> Result<()> {
        let offset = 0;
        let mem_phys_addr = self.udmabuf_acc.phys_addr() as u32;
        ensure!(offset > 15, "allocate err");
        unsafe {
            self.uio_acc.write_mem32(MM2S_START_ADDRESS1 + 4 * offset, mem_phys_addr)
        }
        Ok(())
    }

    pub fn write_mode(&self) {
        unsafe {
            self.uio_acc.write_mem32(MM2S_HSIZE, self.image_width * self.image_bytes_per_pix);
        }
        let mut reg = unsafe {
            self.uio_acc.read_mem32(MM2S_FRMDLY_STRIDE)
        };
        reg &= 0xF << 24;
        reg |= self.image_stride;
        unsafe {
            self.uio_acc.write_mem32(MM2S_FRMDLY_STRIDE, reg);
        }
    }

    pub fn reload(&self) {
        unsafe {
            self.uio_acc.write_mem32(MM2S_VSIZE, self.image_height);
        }
    }

    pub fn stop(&self) {
        unsafe {
            self.uio_acc.write_mem32(MM2S_DMACR, 0x00011080);
        }
        while self.is_running() {}
    }

    pub fn reset(&self) {
        self.stop();
        unsafe {
            self.uio_acc.write_mem32(MM2S_DMACR, 0x00011084);
            while self.uio_acc.read_mem32(MM2S_DMACR) & 4 == 4 {}
        }
    }

    pub fn start(&self) {
        self.allocate();
        self.write_mode();
        self.reload();
        unsafe {
            self.uio_acc.write_mem32(MM2S_DMACR, 0x00011089);
        }
        while !self.is_running() {}
        self.reload();
    }

    pub fn write_frame(&self) {
        unimplemented!();
    }

    fn get_desired_frame(&self) -> u32 {
        unsafe {
            self.uio_acc.read_mem32(PARK_PTR_REG) & 0x1F
        }
    }

    fn set_desired_frame(&self, frame_num: u32) {
        let mut reg = unsafe {
            self.uio_acc.read_mem32(PARK_PTR_REG)
        };
        reg &=  !0x1F;
        reg |=  frame_num;
        unsafe {
            self.uio_acc.write_mem32(PARK_PTR_REG, reg);
        }
    }
}
