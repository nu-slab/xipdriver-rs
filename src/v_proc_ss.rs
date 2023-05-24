use crate::{hwh_parser, mem};
use anyhow::{ensure, Result};

#[cfg(all(target_os = "linux", target_arch = "aarch64"))]
use jelly_mem_access::*;

const BIND_TO: [&str; 1] = ["xilinx.com:ip:v_proc_ss:2.3"];

macro_rules! float2sfix3_12 {
    ($float_num: expr) => {
        (($float_num * 4096.).round() as i32)
    };
}

macro_rules! sfix2f32 {
    ($fix_num: expr) => {
        $fix_num as f32 / 4096.
    };
}

pub struct VideoProcSubsystemCsc {
    pub hwh: hwh_parser::Ip,
    uio_acc: mem::UioAccessor<usize>,
    pub frame_width: u32,
    pub frame_height: u32,
    fmt_in: u32,
    fmt_out: u32,
    color_depth: u32,
    // brightness: u32,
    contrast: i32,
    // saturation: u32,
    // red_gain: u32,
    // green_gain: u32,
    // blue_gain: u32,
    red_offset: i32,
    green_offset: i32,
    blue_offset: i32,
    clamp_min: u32,
    clip_max: u32,
    csc_mat: Vec<f32>,
}

impl VideoProcSubsystemCsc {
    pub fn new(hwh: &hwh_parser::Ip, uio_name: &str) -> Result<Self> {
        ensure!(
            BIND_TO.iter().any(|e| e == &hwh.vlnv),
            "VideoProcSubsystemCsc::new(): This IP is not supported. ({})",
            hwh.vlnv
        );
        let uio = mem::new(uio_name)?;
        let mut csc_mat = vec![0.0; 9];
        csc_mat[0] = 1.;
        csc_mat[4] = 1.;
        csc_mat[8] = 1.;

        Ok(VideoProcSubsystemCsc {
            hwh: hwh.clone(),
            uio_acc: uio,
            frame_width: 1280,
            frame_height: 720,
            fmt_in: 2,
            fmt_out: 0,
            color_depth: 8,
            // brightness: 120,
            contrast: 0,
            // saturation: 100,
            // red_gain: 120,
            // green_gain: 120,
            // blue_gain: 120,
            red_offset: 100,
            green_offset: 100,
            blue_offset: 100,
            clamp_min: 0,
            clip_max: 255,
            csc_mat,
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
        unsafe { self.uio_acc.read_mem32(0x00) & 0x80 == 1 }
    }
    pub fn set_auto_restart_enable(&self, en: bool) {
        let reg = if en { 0x80 } else { 0 };
        unsafe {
            self.uio_acc.write_mem32(0x00, reg);
        }
    }
    pub fn start(&self) -> Result<()> {
        self.configure()?;
        unsafe {
            self.uio_acc.write_mem32(0x00, 0x81);
        }
        Ok(())
    }
    pub fn start_once(&self) -> Result<()> {
        self.configure()?;
        unsafe {
            self.uio_acc.write_mem32(0x00, 0x01);
        }
        Ok(())
    }
    pub fn configure(&self) -> Result<()> {
        self.write_frame_size();
        self.write_fmt()?;
        self.write_csc_matrix();
        Ok(())
    }
    pub fn stop(&self) {
        self.set_auto_restart_enable(false);
    }
    pub fn write_frame_size(&self) {
        unsafe {
            self.uio_acc.write_mem32(0x20, self.frame_width);
            self.uio_acc.write_mem32(0x28, self.frame_height);
            self.uio_acc.write_mem32(0x30, 0);
            self.uio_acc.write_mem32(0x38, self.frame_width-1);
            self.uio_acc.write_mem32(0x40, 0);
            self.uio_acc.write_mem32(0x48, self.frame_height-1);
        }
    }
    pub fn set_format(&mut self, src_fmt: &str, dst_fmt: &str) {
        match (src_fmt, dst_fmt) {
            ("4:2:2", "RGB") | ("BT.709", "RGB") => {
                // 4:2:2 (BT.709) to RGB
                self.csc_mat[0] =  1.1644;
                self.csc_mat[1] =  0.;
                self.csc_mat[2] =  1.7927;
                self.csc_mat[3] =  1.1644;
                self.csc_mat[4] = -0.2132;
                self.csc_mat[5] = -0.5329;
                self.csc_mat[6] =  1.1644;
                self.csc_mat[7] =  2.1124;
                self.csc_mat[8] =  0.;
                self.red_offset = -248;
                self.green_offset = 77;
                self.blue_offset = -289;
                self.fmt_in = 2;
                self.fmt_out = 0;
            },
            ("BT.601", "RGB") => {
                // 4:2:2 (BT.601) to RGB
                self.csc_mat[0] = 1.1644;
                self.csc_mat[1] = 0.;
                self.csc_mat[2] = 1.5906;
                self.csc_mat[3] = 1.1644;
                self.csc_mat[4] = -0.3918;
                self.csc_mat[5] = -0.8130;
                self.csc_mat[6] = 1.1644;
                self.csc_mat[7] = 2.0172;
                self.csc_mat[8] = 0.;
                self.red_offset = -223;
                self.green_offset = 136;
                self.blue_offset = -277;
                self.fmt_in = 2;
                self.fmt_out = 0;
            },
            ("RGB", "4:2:2") | ("RGB", "BT.709") => {
                // RGB to 4:2:2 (BT.709)
                self.csc_mat[0] =  1.1826;
                self.csc_mat[1] =  0.6142;
                self.csc_mat[2] =  0.0620;
                self.csc_mat[3] = -0.1006;
                self.csc_mat[4] = -0.3386;
                self.csc_mat[5] =  0.4392;
                self.csc_mat[6] =  0.4392;
                self.csc_mat[7] = -0.3989;
                self.csc_mat[8] = -0.0403;
                self.red_offset = 16;
                self.green_offset = 128;
                self.blue_offset = 128;
                self.fmt_in = 0;
                self.fmt_out = 2;
            },
            ("RGB", "BT.601") => {
                // RGB to 4:2:2 (BT.601)
                self.csc_mat[0] =  0.2568;
                self.csc_mat[1] =  0.5041;
                self.csc_mat[2] =  0.0979;
                self.csc_mat[3] = -0.1482;
                self.csc_mat[4] = -0.2910;
                self.csc_mat[5] =  0.4393;
                self.csc_mat[6] =  0.4393;
                self.csc_mat[7] = -0.3678;
                self.csc_mat[8] = -0.0714;
                self.red_offset = 16;
                self.green_offset = 128;
                self.blue_offset = 128;
                self.fmt_in = 0;
                self.fmt_out = 2;
            },
            _ => {
                self.csc_mat[0] = 1.;
                self.csc_mat[1] = 0.;
                self.csc_mat[2] = 0.;
                self.csc_mat[3] = 0.;
                self.csc_mat[4] = 1.;
                self.csc_mat[5] = 0.;
                self.csc_mat[6] = 0.;
                self.csc_mat[7] = 0.;
                self.csc_mat[8] = 1.;
                self.red_offset = 0;
                self.green_offset = 0;
                self.blue_offset = 0;
                self.fmt_in = 0;
                self.fmt_out = 0;
            }
        }
        self.clamp_min = 0;
        self.clip_max = (1 << self.color_depth) - 1;
    }
    pub fn write_fmt(&self) -> Result<()> {
        ensure!(self.fmt_in < 4, "fmt_in must be in the range 1 to 3");
        ensure!(self.fmt_out < 4, "fmt_out must be in the range 1 to 3");
        unsafe {
            let reg_src = self.uio_acc.read_mem32(0x10) & 0xFFFFFF00;
            let reg_dst = self.uio_acc.read_mem32(0x18) & 0xFFFFFF00;
            self.uio_acc.write_mem32(0x10, reg_src | self.fmt_in);
            self.uio_acc.write_mem32(0x18, reg_dst | self.fmt_out);
        }
        Ok(())
    }
    pub fn write_csc_matrix(&self) {
        unsafe {
            for i in 0..9 {
                self.uio_acc
                    .write_memi32(0x50 + i * 8, float2sfix3_12!(self.csc_mat[i]));
            }
            self.uio_acc.write_memi32(0x98, self.red_offset);
            self.uio_acc.write_memi32(0xa0, self.green_offset);
            self.uio_acc.write_memi32(0xa8, self.blue_offset);
            self.uio_acc.write_mem32(0xb0, self.clamp_min);
            self.uio_acc.write_mem32(0xb8, self.clip_max);
        }
    }
    pub fn read_csc_matrix(&self) -> [f32; 9] {
        let mut ret = [0.; 9];
        unsafe {
            for i in 0..9 {
                ret[i] = sfix2f32!(self.uio_acc.read_mem32(0x50 + i * 8));
            }
        }
        ret
    }
    pub fn set_contrast(&mut self, value: i32) -> Result<()> {
        let contrast = value * 4 - 200;
        let scale = 1 << (self.color_depth - 8);
        let c_diff = (contrast - self.contrast) * scale;

        self.red_offset += c_diff;
        self.green_offset += c_diff;
        self.blue_offset += c_diff;
        Ok(())
    }
    // pub fn set_saturation(&mut self, value: u32) -> Result<()> {

    //     let saturation = if value == 0 { 1 } else { value * 2 };
    //     let s_diff = saturation / self.saturation;

    //     todo!();
    // }
}
