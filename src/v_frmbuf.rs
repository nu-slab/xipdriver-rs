use crate::{hwh_parser, mem};
use anyhow::{ensure, Result};

#[cfg(all(target_os = "linux", target_arch = "aarch64"))]
use jelly_mem_access::*;

pub struct VideoFrameBufRead {
    pub hwh: hwh_parser::Ip,
    uio_acc: mem::UioAccessor<usize>,
    udmabuf_acc: mem::UdmabufAccessor<usize>,
    fmt: u32,
    pub image_width: u32,
    pub image_height: u32,
    image_bytes_per_pix: u32,
}

impl VideoFrameBufRead {
    pub fn new(hwh: &hwh_parser::Ip, uio_name: &str, udmabuf_name: &str) -> Result<Self> {
        let bind_to: [&str; 1] = ["xilinx.com:ip:v_frmbuf_rd:2.4"];
        ensure!(
            bind_to.iter().any(|e| e == &hwh.vlnv),
            "VideoFrameBufRead::new(): This IP is not supported. ({})",
            hwh.vlnv
        );
        let uio = mem::new(uio_name)?;
        let udmabuf = UdmabufAccessor::new(udmabuf_name, false).unwrap();
        Ok(VideoFrameBufRead {
            hwh: hwh.clone(),
            uio_acc: uio,
            udmabuf_acc: udmabuf,
            fmt: 12,
            image_height: 1280,
            image_width: 720,
            image_bytes_per_pix: 2,
        })
    }

    pub fn is_running(&self) -> bool {
        unsafe { self.uio_acc.read_mem32(0x00) & 1 == 1 }
    }
    pub fn is_done(&self) -> bool {
        unsafe { self.uio_acc.read_mem32(0x00) & 2 == 1 }
    }
    pub fn is_idle(&self) -> bool {
        unsafe { self.uio_acc.read_mem32(0x00) & 4 == 1 }
    }
    pub fn is_ready(&self) -> bool {
        unsafe { self.uio_acc.read_mem32(0x00) & 1 == 0 }
    }
    pub fn get_auto_restart_enable(&self) -> bool {
        unsafe { self.uio_acc.read_mem32(0x00) & 128 == 1 }
    }
    pub fn set_auto_restart_enable(&self, en: bool) {
        let reg = if en { 0x80 } else { 0 };
        unsafe {
            self.uio_acc.write_mem32(0x00, reg);
        }
    }
    pub fn start(&self) {
        let reg = unsafe { self.uio_acc.read_mem32(0x00) } & 0x80;
        unsafe {
            self.uio_acc.write_mem32(0x00, reg | 1);
        }
    }
    pub fn set_framebuf_addr(&self) {
        unsafe {
            self.uio_acc
                .write_mem32(0x30, self.udmabuf_acc.phys_addr() as u32);
        }
    }
    pub fn write_frame<V>(&self, frame: *const V) {
        unsafe {
            self.udmabuf_acc.copy_from(frame, 0x00, 1);
        }
    }
    pub fn set_format(&mut self, fmt: &str) {
        match fmt {
            "YUYV" => {
                self.fmt = 12;
                self.image_bytes_per_pix = 2;
            },
            "RGB8" => {
                self.fmt = 20;
                self.image_bytes_per_pix = 3;
            },
            _ => {
                unimplemented!();
            }
        }
    }
    pub fn get_format(&self) -> &str {
        match self.fmt {
            12 => {
                "YUYV"
            },
            20 => {
                "RGB8"
            },
            _ => {
                unimplemented!();
            }
        }
    }
    pub fn write_format(&self) {
        unsafe {
            self.uio_acc.write_mem32(0x10, self.image_width);
            self.uio_acc.write_mem32(0x18, self.image_height);
            self.uio_acc
                .write_mem32(0x20, self.image_width * self.image_bytes_per_pix);
            self.uio_acc.write_mem32(0x28, self.fmt);
            println!(
                "Width: {}, Height: {}, Stride: {}, Fmt: {}",
                self.uio_acc.read_mem32(0x10),
                self.uio_acc.read_mem32(0x18),
                self.uio_acc.read_mem32(0x20),
                self.uio_acc.read_mem32(0x28)
            )
        }
    }
}

pub struct VideoFrameBufWrite {
    pub hwh: hwh_parser::Ip,
    uio_acc: mem::UioAccessor<usize>,
    udmabuf_acc: mem::UdmabufAccessor<usize>,
    fmt: u32,
    pub image_width: u32,
    pub image_height: u32,
    image_bytes_per_pix: u32,
}

impl VideoFrameBufWrite {
    pub fn new(hwh: &hwh_parser::Ip, uio_name: &str, udmabuf_name: &str) -> Result<Self> {
        let bind_to: [&str; 1] = ["xilinx.com:ip:v_frmbuf_wr:2.4"];
        ensure!(
            bind_to.iter().any(|e| e == &hwh.vlnv),
            "VideoFrameBufRead::new(): This IP is not supported. ({})",
            hwh.vlnv
        );
        let uio = mem::new(uio_name)?;
        let udmabuf = UdmabufAccessor::new(udmabuf_name, false).unwrap();
        Ok(VideoFrameBufWrite {
            hwh: hwh.clone(),
            uio_acc: uio,
            udmabuf_acc: udmabuf,
            fmt: 12,
            image_height: 1280,
            image_width: 720,
            image_bytes_per_pix: 2,
        })
    }

    pub fn is_running(&self) -> bool {
        unsafe { self.uio_acc.read_mem32(0x00) & 1 == 1 }
    }
    pub fn is_done(&self) -> bool {
        unsafe { self.uio_acc.read_mem32(0x00) & 2 == 1 }
    }
    pub fn is_idle(&self) -> bool {
        unsafe { self.uio_acc.read_mem32(0x00) & 4 == 1 }
    }
    pub fn is_ready(&self) -> bool {
        unsafe { self.uio_acc.read_mem32(0x00) & 1 == 0 }
    }
    pub fn get_auto_restart_enable(&self) -> bool {
        unsafe { self.uio_acc.read_mem32(0x00) & 128 == 1 }
    }
    pub fn set_auto_restart_enable(&self, en: bool) {
        let reg = if en { 0x80 } else { 0 };
        unsafe {
            self.uio_acc.write_mem32(0x00, reg);
        }
    }
    pub fn start(&self) {
        let reg = unsafe { self.uio_acc.read_mem32(0x00) } & 0x80;
        unsafe {
            self.uio_acc.write_mem32(0x00, reg | 1);
        }
    }
    pub fn set_framebuf_addr(&self) {
        unsafe {
            self.uio_acc
                .write_mem32(0x30, self.udmabuf_acc.phys_addr() as u32);
        }
    }
    pub fn read_frame(&self) -> image::RgbImage {
        let w = self.image_width as usize;
        let h = self.image_height as usize;
        let bpp = self.image_bytes_per_pix as usize;
        let mut buf = Vec::with_capacity(w * h * bpp);
        println!("len: {}", buf.len());
        unsafe {
            self.udmabuf_acc.copy_to(0x00, buf.as_mut_ptr(),  w * h * bpp);
            buf.set_len(w * h * bpp);
        }
        println!("len: {}", buf.len());
        image::ImageBuffer::from_raw(self.image_width, self.image_height, buf).unwrap()
    }
    pub fn set_format(&mut self, fmt: &str) {
        match fmt {
            "YUYV" => {
                self.fmt = 12;
                self.image_bytes_per_pix = 2;
            },
            "RGB8" => {
                self.fmt = 20;
                self.image_bytes_per_pix = 3;
            },
            _ => {
                unimplemented!();
            }
        }
    }
    pub fn get_format(&self) -> &str {
        match self.fmt {
            12 => {
                "YUYV"
            },
            20 => {
                "RGB8"
            },
            _ => {
                unimplemented!();
            }
        }
    }
    pub fn write_format(&self) {
        unsafe {
            self.uio_acc.write_mem32(0x10, self.image_width);
            self.uio_acc.write_mem32(0x18, self.image_height);
            self.uio_acc
                .write_mem32(0x20, self.image_width * self.image_bytes_per_pix);
            self.uio_acc.write_mem32(0x28, self.fmt);
            println!(
                "Width: {}, Height: {}, Stride: {}, Fmt: {}",
                self.uio_acc.read_mem32(0x10),
                self.uio_acc.read_mem32(0x18),
                self.uio_acc.read_mem32(0x20),
                self.uio_acc.read_mem32(0x28)
            )
        }
    }
}
