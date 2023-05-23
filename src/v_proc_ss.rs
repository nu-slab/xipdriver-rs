
use crate::{hwh_parser, mem};
use anyhow::{ensure, Result};

#[cfg(all(target_os = "linux", target_arch = "aarch64"))]
use jelly_mem_access::*;

const BIND_TO: [&str; 1] = ["xilinx.com:ip:v_proc_ss:2.3"];

macro_rules! float2sfix3_12 {
    ($float_num: expr) => {
        if $float_num >= 0. {
            (($float_num * 4096.).round() as u32) & 0x7FFF
        }
        else {
            ((($float_num * -4096.).round() as u32) | 0x8000) & 0xFFFF
        }
    };
}

macro_rules! sfix2f32 {
    ($fix_num: expr) => {
        if $fix_num & 0x8000 == 0 {
            ($fix_num & 0x7FFF) as f32 / 4096.
        }
        else {
            ($fix_num & 0x7FFF) as f32 / -4096.
        }
    };
}


pub struct VideoProcSubsystemCsc {
    pub hwh: hwh_parser::Ip,
    uio_acc: mem::UioAccessor<usize>,
    pub fmt_in: u32,
    pub fmt_out: u32,
    pub color_depth: u32,
    pub brightness: u32,
    pub contrast: i32,
    pub saturation: u32,
    pub red_gain: u32,
    pub green_gain: u32,
    pub blue_gain: u32,
    pub red_offset: i32,
    pub green_offset: i32,
    pub blue_offset: i32,
    pub clamp_min: u32,
    pub clip_max: u32,
    pub csc_mat: Vec<f32>,
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
            fmt_in: 2,
            fmt_out: 0,
            color_depth: 8,
            brightness: 120,
            contrast: 0,
            saturation: 100,
            red_gain: 120,
            green_gain: 120,
            blue_gain: 120,
            red_offset: 0,
            green_offset: 0,
            blue_offset: 0,
            clamp_min: 0,
            clip_max: 0,
            csc_mat: csc_mat.clone()
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
        self.write_fmt()?;
        self.write_csc_matrix();
        let reg = unsafe { self.uio_acc.read_mem32(0x00) } & 0x80;
        unsafe {
            self.uio_acc.write_mem32(0x00, reg | 1);
        }
        Ok(())
    }
    pub fn set_frame_size(&self, image_width: u32, image_height: u32) {
        unsafe {
            self.uio_acc.write_mem32(0x20, image_width);
            self.uio_acc.write_mem32(0x28, image_height);
            self.uio_acc.write_mem32(0x30, 0);
            self.uio_acc.write_mem32(0x38, image_width-1);
            self.uio_acc.write_mem32(0x40, 0);
            self.uio_acc.write_mem32(0x48, image_height-1);
        }
    }
    pub fn set_mode(&mut self, src_mode: u32, dst_mode: u32) -> Result<()> {
        ensure!(src_mode < 4, "src_mode must be in the range 1 to 3");
        ensure!(dst_mode < 4, "dst_mode must be in the range 1 to 3");
        match (src_mode, dst_mode) {
            (2, 0) => { // 4:2:2 to RGB
                self.csc_mat[0] = 1.1644;
                self.csc_mat[1] = 0.;
                self.csc_mat[2] = 1.5906;
                self.csc_mat[3] = 1.1644;
                self.csc_mat[4] = -0.3918;
                self.csc_mat[5] = -0.8130;
                self.csc_mat[6] = 1.1644;
                self.csc_mat[7] = 2.0172;
                self.csc_mat[8] = 0.;
            }
            _ => {
                todo!();
            }
        }
        self.fmt_in = src_mode;
        self.fmt_out = dst_mode;
        self.clamp_min = 0;
        self.clip_max = (1 << self.color_depth) - 1;
        Ok(())
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
                self.uio_acc.write_mem32(0x50 + i * 8, float2sfix3_12!(self.csc_mat[i]));
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
