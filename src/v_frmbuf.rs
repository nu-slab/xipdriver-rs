use anyhow::{ensure, Result, Context, bail};

#[cfg(all(target_os = "linux", target_arch = "aarch64"))]
use jelly_mem_access::*;

use crate::json_as_map;
use crate::json_as_str;
use crate::json_as_u32;

pub struct VideoFrameBufRead {
    uio_acc: UioAccessor<usize>,
    udmabuf_acc: UdmabufAccessor<usize>,
    fmt_id: u32,
    max_width: u32,
    max_height: u32,
    has_rgb8: bool,
    has_yuyv8: bool,
    pub frame_width: u32,
    pub frame_height: u32,
    pub pix_per_clk: u32,
    bytes_per_pix: u32,
}

impl VideoFrameBufRead {
    pub fn new(hw_info: &serde_json::Value) -> Result<Self> {
        let hw_object = json_as_map!(hw_info);
        let hw_params = json_as_map!(hw_object["params"]);
        let vendor = json_as_str!(hw_object["vendor"]);
        let library = json_as_str!(hw_object["library"]);
        let name = json_as_str!(hw_object["name"]);
        let uio_name = json_as_str!(hw_object["uio"]);
        let udmabuf_name = json_as_str!(hw_object["udmabuf"][0]);
        let max_width: u32 = json_as_u32!(hw_params["MAX_COLS"]);
        let max_height: u32 = json_as_u32!(hw_params["MAX_ROWS"]);
        let has_rgb8 = json_as_u32!(hw_params["HAS_RGB8"]) == 1;
        let has_yuyv8 = json_as_u32!(hw_params["HAS_YUYV8"]) == 1;
        ensure!(
            vendor == "xilinx.com" &&
            library == "ip" &&
            name == "v_frmbuf_rd",
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
        let udmabuf = match UdmabufAccessor::new(udmabuf_name, false) {
            Ok(udmabuf_acc) => {
                udmabuf_acc
            },
            Err(e) => {
                bail!("UdmabufAccessor: {}", e)
            }
        };
        Ok(VideoFrameBufRead {
            uio_acc: uio,
            udmabuf_acc: udmabuf,
            fmt_id: 12,
            max_width,
            max_height,
            has_rgb8,
            has_yuyv8,
            frame_height: 1280,
            frame_width: 720,
            pix_per_clk: 1,
            bytes_per_pix: 2,
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
        unsafe { self.uio_acc.read_mem32(0x00) & 1 == 0 }
    }
    pub fn get_auto_restart_enable(&self) -> bool {
        unsafe { self.uio_acc.read_mem32(0x00) & 0x80 == 0x80 }
    }
    pub fn set_auto_restart_enable(&self, en: bool) {
        let reg = if en { 0x80 } else { 0 };
        unsafe {
            self.uio_acc.write_mem32(0x00, reg);
        }
    }
    pub fn start(&mut self) {
        self.configure();
        unsafe {
            self.uio_acc.write_mem32(0x00, 0x81);
        }
    }
    pub fn start_once(&mut self) {
        self.configure();
        unsafe {
            self.uio_acc.write_mem32(0x00, 0x01);
        }
    }
    pub fn configure(&mut self) {
        self.write_format();
        self.write_framebuf_addr();
    }
    pub fn stop(&self) {
        self.set_auto_restart_enable(false);
        while !self.is_ready() { }
    }
    pub fn wait_done_interrupt(&mut self) {
        self.uio_acc.set_irq_enable(true).unwrap();
        unsafe {
            self.uio_acc.write_mem32(0x04, 0x01);
            self.uio_acc.write_mem32(0x08, 0x01);
        }
        if !self.is_idle() {
            self.uio_acc.wait_irq().unwrap();
        }
        unsafe {
            self.uio_acc.write_mem32(0x0c, 0x01);
            self.uio_acc.write_mem32(0x04, 0x00);
        }
    }
    pub fn write_framebuf_addr(&self) {
        unsafe {
            self.uio_acc
                .write_mem32(0x30, self.udmabuf_acc.phys_addr() as u32);
        }
    }
    pub fn write_frame<V>(&mut self, frame: *const V) {
        let count = if core::mem::size_of::<V>() == 1 {
            (self.frame_width * self.frame_height * self.bytes_per_pix) as usize
        } else {
            1
        };
        self.stop();
        unsafe {
            self.udmabuf_acc.copy_from(frame, 0x00, count);
        }
        self.start();
    }
    pub fn set_format(&mut self, fmt: &str) {
        match fmt {
            "YUYV" => {
                self.fmt_id = 12;
                self.bytes_per_pix = 2;
            }
            "RGB8" => {
                self.fmt_id = 20;
                self.bytes_per_pix = 3;
            }
            _ => {
                unimplemented!();
            }
        }
    }
    pub fn get_format(&self) -> &str {
        match self.fmt_id {
            12 => "YUYV",
            20 => "RGB8",
            _ => {
                unimplemented!();
            }
        }
    }
    pub fn write_format(&self) {
        let mmap_width_bytes = self.pix_per_clk * 8;
        let stride = ((self.frame_width * self.bytes_per_pix + mmap_width_bytes - 1)
            / mmap_width_bytes)
            * mmap_width_bytes;
        unsafe {
            self.uio_acc.write_mem32(0x10, self.frame_width);
            self.uio_acc.write_mem32(0x18, self.frame_height);
            self.uio_acc.write_mem32(0x20, stride);
            self.uio_acc.write_mem32(0x28, self.fmt_id);
        }
    }
}

pub struct VideoFrameBufWrite {
    uio_acc: UioAccessor<usize>,
    udmabuf_acc: UdmabufAccessor<usize>,
    fmt_id: u32,
    max_width: u32,
    max_height: u32,
    has_rgb8: bool,
    has_yuyv8: bool,
    pub frame_width: u32,
    pub frame_height: u32,
    pub pix_per_clk: u32,
    bytes_per_pix: u32,
}

impl VideoFrameBufWrite {
    pub fn new(hw_info: &serde_json::Value) -> Result<Self> {
        let hw_object = json_as_map!(hw_info);
        let hw_params = json_as_map!(hw_object["params"]);
        let vendor = json_as_str!(hw_object["vendor"]);
        let library = json_as_str!(hw_object["library"]);
        let name = json_as_str!(hw_object["name"]);
        let uio_name = json_as_str!(hw_object["uio"]);
        let udmabuf_name = json_as_str!(hw_object["udmabuf"][0]);
        let max_width = json_as_u32!(hw_params["MAX_COLS"]);
        let max_height = json_as_u32!(hw_params["MAX_ROWS"]);
        let has_rgb8 = json_as_u32!(hw_params["HAS_RGB8"]) == 1;
        let has_yuyv8 = json_as_u32!(hw_params["HAS_YUYV8"]) == 1;
        ensure!(
            vendor == "xilinx.com" &&
            library == "ip" &&
            name == "v_frmbuf_wr",
            "VideoFrameBufWrite::new(): This IP is not supported. ({})",
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
        let udmabuf = match UdmabufAccessor::new(udmabuf_name, false) {
            Ok(udmabuf_acc) => {
                udmabuf_acc
            },
            Err(e) => {
                bail!("UdmabufAccessor: {}", e)
            }
        };
        Ok(VideoFrameBufWrite {
            uio_acc: uio,
            udmabuf_acc: udmabuf,
            fmt_id: 12,
            max_width,
            max_height,
            has_rgb8,
            has_yuyv8,
            frame_height: max_width,
            frame_width: max_height,
            pix_per_clk: 1,
            bytes_per_pix: 2,
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
        unsafe { self.uio_acc.read_mem32(0x00) & 1 == 0 }
    }
    pub fn get_auto_restart_enable(&self) -> bool {
        unsafe { self.uio_acc.read_mem32(0x00) & 0x80 == 0x80 }
    }
    pub fn set_auto_restart_enable(&self, en: bool) {
        let reg = if en { 0x80 } else { 0 };
        unsafe {
            self.uio_acc.write_mem32(0x00, reg);
        }
    }
    pub fn start(&self) {
        self.write_format();
        self.set_framebuf_addr();
        unsafe {
            self.uio_acc.write_mem32(0x00, 0x81);
        }
    }
    pub fn start_once(&self) {
        unsafe {
            self.uio_acc.write_mem32(0x00, 0x01);
        }
    }
    pub fn stop(&self) {
        self.set_auto_restart_enable(false);
    }
    pub fn set_framebuf_addr(&self) {
        unsafe {
            self.uio_acc
                .write_mem32(0x30, self.udmabuf_acc.phys_addr() as u32);
        }
    }
    pub fn read_frame_as_image(&self) -> image::RgbImage {
        image::ImageBuffer::from_raw(self.frame_width, self.frame_height, self.read_frame()).unwrap()
    }
    pub fn read_frame(&self) -> Vec<u8> {
        let w = self.frame_width as usize;
        let h = self.frame_height as usize;
        let bpp = self.bytes_per_pix as usize;
        let mut buf = Vec::with_capacity(w * h * bpp);
        unsafe {
            self.udmabuf_acc
                .copy_to(0x00, buf.as_mut_ptr(), w * h * bpp);
            buf.set_len(w * h * bpp);
        }
        buf
    }
    pub fn set_format(&mut self, fmt: &str) {
        match fmt {
            "YUYV" => {
                self.fmt_id = 12;
                self.bytes_per_pix = 2;
            }
            "RGB8" => {
                self.fmt_id = 20;
                self.bytes_per_pix = 3;
            }
            _ => {
                unimplemented!();
            }
        }
    }
    pub fn get_format(&self) -> &str {
        match self.fmt_id {
            12 => "YUYV",
            20 => "RGB8",
            _ => {
                unimplemented!();
            }
        }
    }
    pub fn write_format(&self) {
        let mmap_width_bytes = self.pix_per_clk * 8;
        let stride = ((self.frame_width * self.bytes_per_pix + mmap_width_bytes - 1)
            / mmap_width_bytes)
            * mmap_width_bytes;
        unsafe {
            self.uio_acc.write_mem32(0x10, self.frame_width);
            self.uio_acc.write_mem32(0x18, self.frame_height);
            self.uio_acc.write_mem32(0x20, stride);
            self.uio_acc.write_mem32(0x28, self.fmt_id);
        }
    }
}
