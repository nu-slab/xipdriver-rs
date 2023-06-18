#![allow(unused)]

use anyhow::{ensure, Result, Context, bail};

#[cfg(all(target_os = "linux", target_arch = "aarch64"))]
use jelly_mem_access::*;


use crate::json_as_map;
use crate::json_as_vec;
use crate::json_as_str;

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

pub struct AxiVdmaMM2S {
    uio_acc: UioAccessor<usize>,
    udmabuf_acc: Vec<UdmabufAccessor<usize>>,
    pub frame_width: u32,
    pub frame_height: u32,
    pub bytes_per_pix: u32,
    pub pix_per_clk: u32,
    desired_frame: usize,
    frame_buffers: usize,
}

impl AxiVdmaMM2S {
    pub fn new(hw_info: &serde_json::Value) -> Result<Self> {
        let hw_object = json_as_map!(hw_info);
        // let hw_params = json_as_map!(hw_object["params"]);
        let vendor = json_as_str!(hw_object["vendor"]);
        let library = json_as_str!(hw_object["library"]);
        let name = json_as_str!(hw_object["name"]);
        let uio_name = json_as_str!(hw_object["uio"]);
        let udmabuf_names = json_as_vec!(hw_object["udmabuf"]);
        ensure!(
            vendor == "xilinx.com" &&
            library == "ip" &&
            name == "axi_vdma",
            "AxiVdmaMM2S::new(): This IP is not supported. ({})",
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
        let mut udmabuf = Vec::new();
        for name in udmabuf_names.iter() {
            let udmabuf_name = name.as_str().context("udmabuf_name is not string")?;
            match UdmabufAccessor::new(udmabuf_name, false) {
                Ok(udmabuf_acc) => {
                    udmabuf.push(udmabuf_acc);
                },
                Err(e) => {
                    bail!("UdmabufAccessor: {}", e)
                }
            };
        }

        Ok(AxiVdmaMM2S {
            uio_acc: uio,
            udmabuf_acc: udmabuf,
            frame_height: 1280,
            frame_width: 720,
            bytes_per_pix: 3,
            pix_per_clk: 1,
            desired_frame: 1,
            frame_buffers: 3,
        })
    }

    pub fn is_running(&self) -> bool {
        unsafe { self.uio_acc.read_mem32(MM2S_DMASR) & 1 == 0 }
    }

    fn write_framebuf_addr(&self) -> Result<()> {
        ensure!(self.frame_buffers < 16, "self.frame_buffers > 15");
        for i in 0..self.frame_buffers {
            let mem_phys_addr = self.udmabuf_acc[i].phys_addr() as u32;
            unsafe {
                self.uio_acc
                    .write_mem32(MM2S_START_ADDRESS1 + 4 * i, mem_phys_addr);
            }
        }

        Ok(())
    }

    pub fn write_format(&self) {
        let mmap_width_bytes = self.pix_per_clk * 8;
        let stride = ((self.frame_width * self.bytes_per_pix + mmap_width_bytes - 1)
            / mmap_width_bytes)
            * mmap_width_bytes;
        unsafe {
            self.uio_acc
                .write_mem32(MM2S_HSIZE, self.frame_width * self.bytes_per_pix);
        }
        let mut reg = unsafe { self.uio_acc.read_mem32(MM2S_FRMDLY_STRIDE) };
        reg &= 0xF << 24;
        reg |= stride;
        unsafe {
            self.uio_acc.write_mem32(MM2S_FRMDLY_STRIDE, reg);
        }
    }

    pub fn reload(&self) {
        unsafe {
            self.uio_acc.write_mem32(MM2S_VSIZE, self.frame_height);
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
            self.uio_acc.write_mem32(MM2S_DMACR, 0x00011084);
        }
    }

    pub fn start(&mut self) -> Result<()> {
        self.write_framebuf_addr()?;
        self.write_format();
        self.reload();
        unsafe {
            self.uio_acc.write_mem32(MM2S_DMACR, 0x00011089);
        }
        while !self.is_running() {}
        self.reload();
        self.uio_acc.set_irq_enable(true).unwrap();
        Ok(())
    }
    pub fn write_frame<V>(&mut self, frame: *const V) -> Result<()> {
        ensure!(self.frame_buffers < 16, "self.frame_buffers > 15");
        while unsafe { self.uio_acc.read_mem32(MM2S_DMASR) & 0x1000 == 0 } {
            self.uio_acc.wait_irq().unwrap();
        }
        unsafe {
            self.uio_acc.write_mem32(MM2S_DMASR, 0x1000);
        }
        self.inc_desired_frame();
        self.write_frame_internal(frame);
        self.reload();
        self.write_desired_frame();
        println!("{}", self.desired_frame);
        Ok(())
    }
    fn write_frame_internal<V>(&mut self, frame: *const V) {
        let count = if core::mem::size_of::<V>() == 1 {
            (self.frame_width * self.frame_height * self.bytes_per_pix) as usize
        } else {
            1
        };
        unsafe {
            self.udmabuf_acc[self.desired_frame].copy_from(frame, 0, count);
        }
    }

    pub fn read_desired_frame(&self) -> u32 {
        unsafe { self.uio_acc.read_mem32(PARK_PTR_REG) & 0x1F }
    }
    pub fn read_active_frame(&self) -> u32 {
        unsafe { (self.uio_acc.read_mem32(PARK_PTR_REG) >> 16) & 0x1F }
    }

    fn write_desired_frame(&self) {
        let mut reg = unsafe { self.uio_acc.read_mem32(PARK_PTR_REG) };
        reg &= !0x1F;
        reg |= self.desired_frame as u32;
        unsafe {
            self.uio_acc.write_mem32(PARK_PTR_REG, reg);
        }
    }
    fn inc_desired_frame(&mut self) {
        self.desired_frame = (self.read_desired_frame() as usize + 1) % self.frame_buffers;
    }
}
