use anyhow::{bail, ensure, Context, Result};

use crate::json_as_map;
use crate::json_as_str;
use crate::json_as_vec;

use jelly_mem_access::*;

pub struct BirdEyeViewHW {
    uio_acc: UioAccessor<usize>,
    udmabuf_acc: Vec<UdmabufAccessor<usize>>,
    max_width: u32,
    max_height: u32,
}

impl BirdEyeViewHW {
    pub fn new(hw_info: &serde_json::Value) -> Result<Self> {
        let hw_object = json_as_map!(hw_info);
        //let hw_params = json_as_map!(hw_object["params"]);
        let vendor = json_as_str!(hw_object["vendor"]);
        let library = json_as_str!(hw_object["library"]);
        let name = json_as_str!(hw_object["name"]);
        let uio_name = json_as_str!(hw_object["uio"]);
        let udmabuf_names = json_as_vec!(hw_object["udmabuf"]);

        ensure!(
            vendor == "xilinx.com"
                && (library == "ip" || library == "hls")
                && name == "bird_eye_view",
            "BierdEyeViewHW::new(): This IP is nort supported. ({})",
            name
        );

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
            udmabuf_acc: udmabuf,
            max_width: 1280,
            max_height: 720,
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
            self.uio_acc
                .write_mem32(0x18, self.udmabuf_acc[0].phys_addr() as u32);
        }
        Ok(())
    }

    pub fn set_img_map_addr(&mut self) -> Result<()> {
        unsafe {
            self.uio_acc
                .write_mem32(0x24, self.udmabuf_acc[1].phys_addr() as u32);
        }
        Ok(())
    }

    pub fn set_img_out_addr(&mut self) -> Result<()> {
        unsafe {
            self.uio_acc
                .write_mem32(0x30, self.udmabuf_acc[2].phys_addr() as u32);
        }
        Ok(())
    }

    pub fn write_img_in(&mut self, img_in: &[u32]) -> Result<()> {
        ensure!(
            img_in.len() <= self.udmabuf_acc[0].size(),
            "img_in.len() is too large\n img_in.len() : {}\n udmabuf : {}",
            img_in.len(),
            self.udmabuf_acc[0].size()
        );
        unsafe {
            self.udmabuf_acc[0].copy_from(img_in.as_ptr(), 0x00, img_in.len());
        }
        Ok(())
    }

    pub fn write_img_map(&mut self, img_map: &[u32]) -> Result<()> {
        ensure!(
            img_map.len() <= self.udmabuf_acc[1].size(),
            "img_map.len() is too large\n img_map.len() : {}\n udmabuf : {}",
            img_map.len(),
            self.udmabuf_acc[1].size()
        );
        unsafe {
            self.udmabuf_acc[1].copy_from(img_map.as_ptr(), 0x00, img_map.len());
        }
        Ok(())
    }

    pub fn read_img_out(&mut self) -> Result<Vec<u32>> {
        let w = self.max_width as usize;
        let h = self.max_height as usize;
        let mut buf = Vec::with_capacity(w * h);
        unsafe {
            self.udmabuf_acc[2].copy_to(0x00, buf.as_mut_ptr(), w * h);
            buf.set_len(w * h);
        }
        Ok(buf)
    }
}
