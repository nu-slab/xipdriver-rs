use anyhow::{ensure, Result, Context, bail};

use jelly_mem_access::*;

use crate::json_as_map;
use crate::json_as_str;
use crate::json_as_u32;

#[derive(Clone, Copy, Debug)]
pub enum ColorFormat {
    /* Video in memory formats */
    Rgbx8 = 10,   // [31:0] x:B:G:R 8:8:8:8
    Yuvx8,        // [31:0] x:V:U:Y 8:8:8:8
    Yuyv8,        // [31:0] V:Y:U:Y 8:8:8:8
    Rgba8,        // [31:0] A:B:G:R 8:8:8:8
    Yuva8,        // [31:0] A:V:U:Y 8:8:8:8
    Rgbx10,       // [31:0] x:B:G:R 2:10:10:10
    Yuvx10,       // [31:0] x:V:U:Y 2:10:10:10


    Rgb565,       // [15:0] B:G:R 5:6:5
    YUv8,         // [15:0] Y:Y 8:8, [15:0] V:U 8:8
    YUv8420,      // [15:0] Y:Y 8:8, [15:0] V:U 8:8
    Rgb8,         // [23:0] B:G:R 8:8:8
    Yuv8,         // [24:0] V:U:Y 8:8:8
    YUv10,        // [31:0] x:Y:Y:Y 2:10:10:10 [31:0] x:U:V:U 2:10:10:10
    YUv10_420,    // [31:0] x:Y:Y:Y 2:10:10:10 [31:0] x:U:V:U 2:10:10:10
    Y8,           // [31:0] Y:Y:Y:Y 8:8:8:8
    Y10,          // [31:0] x:Y:Y:Y 2:10:10:10
    Bgra8,        // [31:0] A:R:G:B 8:8:8:8
    Bgrx8,        // [31:0] X:R:G:B 8:8:8:8
    Uyvy8,        // [31:0] Y:V:Y:U 8:8:8:8
    Bgr8,         // [23:0] R:G:B 8:8:8
    Rgbx12,       // [39:0] x:R:G:B 4:12:12:12
    Yuvx12,       // [39:0] x:V:U:Y 4:12:12:12
    YUv12,        // [23:0] Y:Y 12:12, [23:0] V:U 12:12
    YUv12_420,     // [23:0] Y:Y 12:12, [23:0] V:U 12:12
    Y12,          // [39:0] x:Y2:Y1:Y0 4:12:12:12
    Rgb16,        // [47:0] R:G:B 16:16:16
    Yuv16,        // [47:0] V:U:Y 16:16:16
    YUv16,        // [31:0] Y:Y 16:16, [31:0] V:U 16:16
    Yuv16_420,     // [31:0] Y:Y 16:16, [31:0] V:U 16:16
    Y16,          // [47:0] Y2:Y1:Y0 16:16:16
    RGb8,         // [7:0] R:8, [7:0] G:8, [7:0] B:8
    YUV8_420,      // [15:0] Y:Y 8:8, [7:0] U:8, [7:0] V:8
    YUV8,         // [7:0] Y:8, [7:0] U:8, [7:0] V:8
    YUV10,       // [9:0] Y:10, [9:0] U:10, [9:0] V:10
}


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
    pix_per_clk: u32,
    bytes_per_pix: u32,
    tie_en: bool,
    tie_addr: usize,
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
        let pix_per_clk = json_as_u32!(hw_params["SAMPLES_PER_CLOCK"]);
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
            fmt_id: 0,
            max_width,
            max_height,
            has_rgb8,
            has_yuyv8,
            frame_height: max_height,
            frame_width: max_width,
            pix_per_clk,
            bytes_per_pix: 0,
            tie_en: false,
            tie_addr: 0,
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
    pub fn start(&mut self) -> Result<()> {
        self.configure()?;
        unsafe {
            self.uio_acc.write_mem32(0x00, 0x81);
        }
        Ok(())
    }
    pub fn start_once(&mut self) -> Result<()> {
        self.configure()?;
        unsafe {
            self.uio_acc.write_mem32(0x00, 0x01);
        }
        Ok(())
    }
    pub fn configure(&mut self) -> Result<()> {
        self.write_format()?;
        self.write_framebuf_addr();
        Ok(())
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
            if self.tie_en {
                self.uio_acc.write_mem32(0x30, self.tie_addr as u32);
            }
            else {
                self.uio_acc.write_mem32(0x30, self.udmabuf_acc.phys_addr() as u32);
            }
        }
    }
    pub fn write_frame<V>(&mut self, frame: *const V) -> Result<()> {
        ensure!(self.frame_width <= self.max_width, "FRAME_WIDTH too large");
        ensure!(self.frame_height <= self.max_height, "FRAME_HEIGHT too large");
        let size_of_v = core::mem::size_of::<V>();
        let count = (self.frame_width * self.frame_height * self.bytes_per_pix) as usize;
        ensure!(size_of_v * count < self.udmabuf_acc.phys_addr(), "Array size too large");
        ensure!(size_of_v == 1, "Unsupported data format: {}", size_of_v);
        // self.stop();
        unsafe {
            self.udmabuf_acc.copy_from(frame, 0x00, count);
        }
        self.start()
    }
    pub fn set_format(&mut self, fmt: &str) -> Result<()> {
        match fmt {
            "YUYV" => {
                ensure!(self.has_yuyv8, "YUYV8 is not enabled");
                self.fmt_id = 12;
                self.bytes_per_pix = 2;
            }
            "RGB8" => {
                ensure!(self.has_rgb8, "RGB8 is not enabled");
                self.fmt_id = 20;
                self.bytes_per_pix = 3;
            }
            _ => {
                bail!("{} is not enabled", fmt);
            }
        }
        Ok(())
    }
    pub fn get_format(&self) -> Result<&str> {
        match self.fmt_id {
             0 => Ok("Not Set"),
            12 => Ok("YUYV"),
            20 => Ok("RGB8"),
            _ => {
                bail!("unknown fmt: {}", self.fmt_id);
            }
        }
    }
    pub fn write_format(&self) -> Result<()> {
        ensure!(self.frame_width <= self.max_width, "FRAME_WIDTH too large");
        ensure!(self.frame_height <= self.max_height, "FRAME_HEIGHT too large");
        ensure!(self.fmt_id != 0, "Format is not set");
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
        Ok(())
    }
    pub fn tie(&mut self, vfbw: &VideoFrameBufWrite) {
        self.tie_en = true;
        self.tie_addr = vfbw.get_addr();
    }
    pub fn untie(&mut self) {
        self.tie_en = false;
    }
    pub fn calc_stride(&self, fmt: ColorFormat) -> u32 {
        let mm_width_bytes = self.pix_per_clk * 8;

        let (bpp_numerator, bpp_denominator) = match fmt {
            ColorFormat::YUv10 | ColorFormat::YUv10_420 | ColorFormat::Y10 => {
                (4, 3)
            },
            ColorFormat::YUv8 | ColorFormat::YUv8420 | ColorFormat::Y8 | ColorFormat::YUV10 => {
                (1, 1)
            },
            ColorFormat::Rgb8 | ColorFormat::Yuv8 | ColorFormat::Bgr8 => {
                (3, 1)
            },
            ColorFormat::Rgbx12 | ColorFormat::Yuvx12 => {
                (5, 1)
            },
            ColorFormat::YUv12 | ColorFormat::YUv12_420 | ColorFormat::Y12 => {
                (3, 2)
            },
            ColorFormat::Rgb16 | ColorFormat::Yuv16 => {
                (6, 1)
            },
            ColorFormat::Yuyv8 | ColorFormat::Uyvy8 | ColorFormat::YUv16 | ColorFormat::Yuv16_420 | ColorFormat::Y16 => {
                 (2, 1)
            },
            _ => {
                (4, 1)
            }
        };

        ((((self.frame_width * bpp_numerator) / bpp_denominator) + mm_width_bytes - 1) / mm_width_bytes) * mm_width_bytes
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
        let pix_per_clk = json_as_u32!(hw_params["SAMPLES_PER_CLOCK"]);
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
            fmt_id: 0,
            max_width,
            max_height,
            has_rgb8,
            has_yuyv8,
            frame_height: max_height,
            frame_width: max_width,
            pix_per_clk,
            bytes_per_pix: 0,
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
    pub fn start(&self) -> Result<()> {
        self.write_format()?;
        self.set_framebuf_addr();
        unsafe {
            self.uio_acc.write_mem32(0x00, 0x81);
        }
        Ok(())
    }
    pub fn start_once(&self) -> Result<()> {
        self.write_format()?;
        unsafe {
            self.uio_acc.write_mem32(0x00, 0x01);
        }
        Ok(())
    }
    pub fn stop(&self) {
        self.set_auto_restart_enable(false);
        // while !self.is_ready() { }
    }
    pub fn set_framebuf_addr(&self) {
        unsafe {
            self.uio_acc
                .write_mem32(0x30, self.udmabuf_acc.phys_addr() as u32);
        }
    }
    pub fn read_frame_as_image(&self) -> Result<image::RgbImage> {
        image::ImageBuffer::from_raw(self.frame_width, self.frame_height, self.read_frame()?).context("Can't convert to image")
    }
    pub fn read_frame(&self) -> Result<Vec<u8>> {
        ensure!(self.frame_width <= self.max_width, "FRAME_WIDTH too large");
        ensure!(self.frame_height <= self.max_height, "FRAME_HEIGHT too large");
        let w = self.frame_width as usize;
        let h = self.frame_height as usize;
        let bpp = self.bytes_per_pix as usize;
        let mut buf = Vec::with_capacity(w * h * bpp);
        self.stop();
        unsafe {
            self.udmabuf_acc
                .copy_to(0x00, buf.as_mut_ptr(), w * h * bpp);
            buf.set_len(w * h * bpp);
        }
        self.start()?;
        Ok(buf)
    }
    pub fn set_format(&mut self, fmt: &str) -> Result<()> {
        match fmt {
            "YUYV" => {
                ensure!(self.has_yuyv8, "YUYV8 is not enabled");
                self.fmt_id = 12;
                self.bytes_per_pix = 2;
            }
            "RGB8" => {
                ensure!(self.has_rgb8, "RGB8 is not enabled");
                self.fmt_id = 20;
                self.bytes_per_pix = 3;
            }
            _ => {
                bail!("{} is not enabled", fmt);
            }
        }
        Ok(())
    }
    pub fn get_format(&self) -> Result<&str> {
        match self.fmt_id {
             0 => Ok("Not Set"),
            12 => Ok("YUYV"),
            20 => Ok("RGB8"),
            _ => {
                bail!("unknown fmt: {}", self.fmt_id);
            }
        }
    }
    pub fn write_format(&self) -> Result<()> {
        ensure!(self.frame_width <= self.max_width, "FRAME_WIDTH too large");
        ensure!(self.frame_height <= self.max_height, "FRAME_HEIGHT too large");
        ensure!(self.fmt_id != 0, "Format is not set");
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
        Ok(())
    }
    pub fn get_addr(&self) -> usize {
        self.udmabuf_acc.phys_addr()
    }
}
