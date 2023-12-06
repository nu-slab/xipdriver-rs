use anyhow::{bail, ensure, Context, Result};

use crate::json_as_map;
use crate::json_as_str;
use crate::json_as_u32;
use jelly_mem_access::*;

pub struct BirdEyeViewHW {
    uio_acc: UioAccessor<usize>,
    udmabuf_acc: Vec<UdmabufAccessor<usize>>,
    fmt_id: u32,
    max_width: u32,
    max_height: u32,
    pub frame_width: u32,
    pub frame_height: u32,
    bytes_per_pix: u32,
}

impl BirdEyeViewHW {
    pub fn new(hw_info: &serd_json::Value) -> Result<Self> {
        let hw_object = json_as_map!(hw_info);
        let hw_params = json_as_map!(hw_object["params"]);
        let vendor = json_as_str!(hw_object["vendor"]);
        let library = json_as_str!(hw_object["library"]);
        let name = json_as_str!(hw_object["name"]);
        let uio_name = json_as_str!(hw_object["uio"]);
        let max_width: u32 = json_as_u32!(hw_params["MAX_COLS"]);
        let max_height: u32 = json_as_u32!(hw_params["MAX_ROWS"]);
        let udmabuf_names = json_as_vec!(hw_object["udmabuf"]);

        let uio = match UioAccessor::<usize>::new_with_name(uio_name) {
            Ok(uio_acc) => uio_acc,
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
                }
                Err(e) => {
                    bail!("UdmabufAccessor: {}", e)
                }
            };
        }

        Ok(BirdEyeViewHW {
            uio_acc: uio,
            fmt_id: 0,
            max_width: 1280,
            max_height: 720,
            frame_height: max_height,
            frame_width: max_width,
            bytes_per_pix: 4,
        })
    }

    pub fn is_running(&self) -> bool {
        unsafe { self.uio_acc.read_mem32(0x00) & 1 == 1 }
    }

    pub fn is_done(&self) -> bool {
        unsafe { self.uio_acc.read_mem32(0x00) & 2 == 2 }
    }
    pub fn is_idle(&self) -> bool {
        unsafe { self.uio_acc.read_mem32(0x00) & 4 == 4 }
    }
    pub fn is_ready(&self) -> bool {
        unsafe { self.uio_acc.read_mem32(0x00) & 8 == 8 }
    }

    pub fn get_auto_restart_enable(&self) -> bool {
        unsafe { self.uio_acc.read_mem32(0x00) & 0x20 == 0x20 }
    }
    pub fn set_auto_restart_enable(&self, en: bool) {
        let reg = if en { 0x20 } else { 0 };
        unsafe {
            self.uio_acc.write_mem32(0x00, reg);
        }
    }
    pub fn start_once(&mut self) -> Result<()> {
        unsafe {
            self.uio_acc.write_mem32(0x00, 0x01);
        }
        Ok(())
    }
    pub fn set_img_in_addr(&mut self) -> Result<()> {
        unsafe {
            self.write_mem32(0x18, self.udmabuf_acc[0].phys_addr() as u32);
        }
    }

    pub fn set_img_out_addr(&mut self) -> Result<()> {
        unsafe {
            self.write_mem32(0x30, self.udmabuf_acc[1].phys_addr() as u32);
        }
    }

    pub fn set_img_map_addr(&mut self) -> Result<()> {
        unsafe {
            self.write_mem32(0x24, self.udmabuf_acc[2].phys_addr() as u32);
        }
    }

    pub fn map_img_in(&mut self, img_in: &[u32]) -> Result<()> {
        unsafe {
            self.udmabuf_acc[0].copy_from(img_in.as_ptr, 0x00, img_in.len());
        }
    }

    pub fn map_img_map(&mut self, img_map: &[u32]) -> Result<()> {
        unsafe {
            self.udmabuf_acc[1].copy_from(img_map.as_ptr, 0x00, img_map.len());
        }
    }

    pub fn map_img_out(&mut self) -> Result<(Vec<u32>)> {
        let w = self.frame_width as usize;
        let h = self.frame_height as usize;
        let bpp = self.bytes_per_pix as usize;
        let mut buf = Vec::with_capacity(w * h * bpp);
        unsafe {
            self.udmabuf_acc[2].copy_to(0x00, buf.as_mut_ptr, w * h * bpp);
            buf.set_len(w * h * bpp);
        }
        Ok(buf)
    }
}
