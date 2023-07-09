use anyhow::{ensure, Result, Context, bail};

#[cfg(all(target_os = "linux", target_arch = "aarch64"))]
use jelly_mem_access::*;

use crate::json_as_map;
use crate::json_as_str;
use crate::json_as_i32;
use crate::json_as_f32;

const BRAKE            :usize = 0x00;
const ACCEL_R          :usize = 0x04;
const ACCEL_L          :usize = 0x08;
const KP_R             :usize = 0x0C;
const KI_R             :usize = 0x10;
const KD_R             :usize = 0x14;
const BIAS_R           :usize = 0x18;
const KP_L             :usize = 0x1C;
const KI_L             :usize = 0x20;
const KD_L             :usize = 0x24;
const BIAS_L           :usize = 0x28;
const ROTATION_R       :usize = 0x2C;
const ROTATION_L       :usize = 0x30;
const ROTATION_RESET   :usize = 0x34;
const TOTAL_ROTATION_R :usize = 0x38;
const TOTAL_ROTATION_L :usize = 0x3C;

const FIXED_DECIMAL_BITW: i32 = 16;


pub struct UmvMotorController {
    uio_acc: UioAccessor<usize>,
    accel_max: i32,
    fb_edge_period: f32,
}

impl UmvMotorController {
    pub fn new(hw_info: &serde_json::Value) -> Result<Self> {
        let hw_object = json_as_map!(hw_info);
        let hw_params = json_as_map!(hw_object["params"]);
        let vendor = json_as_str!(hw_object["vendor"]);
        let library = json_as_str!(hw_object["library"]);
        let name = json_as_str!(hw_object["name"]);
        let uio_name = json_as_str!(hw_object["uio"]);
        let accel_max = json_as_i32!(hw_params["ACCEL_MAX"]);
        let fb_edge_period = json_as_f32!(hw_params["FB_EDGE_PERIOD"]);
        ensure!(
            vendor == "slab" &&
            library == "umv_project" &&
            name == "umv_motor_controller",
            "UmvMotorController::new(): This IP is not supported. ({})",
            name
        );
        let uio_acc = match UioAccessor::<usize>::new_with_name(uio_name) {
            Ok(uio_acc) => {
                uio_acc
            },
            Err(e) => {
                bail!("UioAccessor: {}", e)
            }
        };
        Ok(UmvMotorController {
            uio_acc,
            accel_max,
            fb_edge_period,
        })
    }
    pub fn write_brake(&self, val: bool) {
        let val_u32 = if val { 1 } else { 0 };
        unsafe { self.uio_acc.write_mem32(BRAKE, val_u32); }
    }
    pub fn write_accel_right(&self, val: i32) -> Result<()> {
        ensure!(val <= self.accel_max, "accel_right must be less than {}", self.accel_max);
        unsafe { self.uio_acc.write_memi32(ACCEL_R, val); }
        Ok(())
    }
    pub fn write_accel_left(&self, val: i32) -> Result<()> {
        ensure!(val <= self.accel_max, "accel_left must be less than {}", self.accel_max);
        unsafe { self.uio_acc.write_memi32(ACCEL_L, val); }
        Ok(())
    }
    pub fn write_accel(&self, left_val: i32, right_val: i32) -> Result<()> {
        self.write_accel_left(left_val)?;
        self.write_accel_right(right_val)
    }
    pub fn set_accel_rpm_left(&self, val: f32) -> Result<()> {
        let acc_val  = (val  * 360. * self.fb_edge_period / 60.).floor() as i32;
        self.write_accel_left(acc_val)
    }
    pub fn set_accel_rpm_right(&self, val: f32) -> Result<()> {
        let acc_val  = (val  * 360. * self.fb_edge_period / 60.).floor() as i32;
        self.write_accel_right(acc_val)
    }
    pub fn set_accel_rpm(&self, left_val: f32, right_val: f32) -> Result<()> {
        self.set_accel_rpm_left(left_val)?;
        self.set_accel_rpm_right(right_val)
    }
    pub fn write_kp_right(&self, val: f32) -> Result<()> {
        ensure!(val >= 0., "Kp_right must be a positive number");
        ensure!(val < (2.0_f32).powi(FIXED_DECIMAL_BITW), "Kp_right must be less than {}", (2.0_f32).powi(FIXED_DECIMAL_BITW));
        let fixed_val = (val * (2.0_f32).powi(FIXED_DECIMAL_BITW)).floor() as u32;
        unsafe { self.uio_acc.write_mem32(KP_R, fixed_val); }
        Ok(())
    }
    pub fn write_ki_right(&self, val: f32) -> Result<()> {
        ensure!(val >= 0., "Ki_right must be a positive number");
        ensure!(val < (2.0_f32).powi(FIXED_DECIMAL_BITW), "Ki_right must be less than {}", (2.0_f32).powi(FIXED_DECIMAL_BITW));
        let fixed_val = (val * (2.0_f32).powi(FIXED_DECIMAL_BITW)).floor() as u32;
        unsafe { self.uio_acc.write_mem32(KI_R, fixed_val); }
        Ok(())
    }
    pub fn write_kd_right(&self, val: f32) -> Result<()> {
        ensure!(val >= 0., "Kd_right must be a positive number");
        ensure!(val < (2.0_f32).powi(FIXED_DECIMAL_BITW), "Kd_right must be less than {}", (2.0_f32).powi(FIXED_DECIMAL_BITW));
        let fixed_val = (val * (2.0_f32).powi(FIXED_DECIMAL_BITW)).floor() as u32;
        unsafe { self.uio_acc.write_mem32(KD_R, fixed_val); }
        Ok(())
    }
    pub fn write_bias_right(&self, val: f32) -> Result<()> {
        ensure!(val >= 0., "Bias_right must be a positive number");
        ensure!(val < (2.0_f32).powi(FIXED_DECIMAL_BITW), "Bias_right must be less than {}", (2.0_f32).powi(FIXED_DECIMAL_BITW));
        let fixed_val = (val * (2.0_f32).powi(FIXED_DECIMAL_BITW)).floor() as u32;
        unsafe { self.uio_acc.write_mem32(BIAS_R, fixed_val); }
        Ok(())
    }
    pub fn write_kp_left(&self, val: f32) -> Result<()> {
        ensure!(val >= 0., "Kp_left must be a positive number");
        ensure!(val < (2.0_f32).powi(FIXED_DECIMAL_BITW), "Kp_left must be less than {}", (2.0_f32).powi(FIXED_DECIMAL_BITW));
        let fixed_val = (val * (2.0_f32).powi(FIXED_DECIMAL_BITW)).floor() as u32;
        unsafe { self.uio_acc.write_mem32(KP_L, fixed_val); }
        Ok(())
    }
    pub fn write_ki_left(&self, val: f32) -> Result<()> {
        ensure!(val >= 0., "Ki_left must be a positive number");
        ensure!(val < (2.0_f32).powi(FIXED_DECIMAL_BITW), "Ki_left must be less than {}", (2.0_f32).powi(FIXED_DECIMAL_BITW));
        let fixed_val = (val * (2.0_f32).powi(FIXED_DECIMAL_BITW)).floor() as u32;
        unsafe { self.uio_acc.write_mem32(KI_L, fixed_val); }
        Ok(())
    }
    pub fn write_kd_left(&self, val: f32) -> Result<()> {
        ensure!(val >= 0., "Kd_left must be a positive number");
        ensure!(val < (2.0_f32).powi(FIXED_DECIMAL_BITW), "Kd_left must be less than {}", (2.0_f32).powi(FIXED_DECIMAL_BITW));
        let fixed_val = (val * (2.0_f32).powi(FIXED_DECIMAL_BITW)).floor() as u32;
        unsafe { self.uio_acc.write_mem32(KD_L, fixed_val); }
        Ok(())
    }
    pub fn write_bias_left(&self, val: f32) -> Result<()> {
        ensure!(val >= 0., "Bias_left must be a positive number");
        ensure!(val < (2.0_f32).powi(FIXED_DECIMAL_BITW), "Bias_left must be less than {}", (2.0_f32).powi(FIXED_DECIMAL_BITW));
        let fixed_val = (val * (2.0_f32).powi(FIXED_DECIMAL_BITW)).floor() as u32;
        unsafe { self.uio_acc.write_mem32(BIAS_L, fixed_val); }
        Ok(())
    }
    pub fn set_kp(&self, val: f32) -> Result<()> {
        self.write_kp_left(val)?;
        self.write_kp_right(val)
    }
    pub fn set_ki(&self, val: f32) -> Result<()> {
        self.write_ki_left(val)?;
        self.write_ki_right(val)
    }
    pub fn set_kd(&self, val: f32) -> Result<()> {
        self.write_kd_left(val)?;
        self.write_kd_right(val)
    }
    pub fn set_bias(&self, val: f32) -> Result<()> {
        self.write_bias_left(val)?;
        self.write_bias_right(val)
    }
    pub fn read_rotation_right(&self) -> i32 {
        unsafe { self.uio_acc.read_memi32(ROTATION_R) }
    }
    pub fn read_rotation_left(&self) -> i32 {
        unsafe { self.uio_acc.read_memi32(ROTATION_L) }
    }
    pub fn get_wheel_rpm_right(&self) -> f32 {
        // degree per (self.fb_edge_period) sec.
        let lotation = self.read_rotation_right() as f32;
        lotation * 60. / self.fb_edge_period / 360.
    }
    pub fn get_wheel_rpm_left(&self) -> f32 {
        // degree per (self.fb_edge_period) sec.
        let lotation = self.read_rotation_left() as f32;
        lotation * 60. / self.fb_edge_period / 360.
    }
    pub fn reset_total_rotation(&self) {
        unsafe { self.uio_acc.write_mem32(ROTATION_RESET, 1); }
    }
    pub fn read_total_rotation_right(&self) -> i32 {
        unsafe { self.uio_acc.read_memi32(TOTAL_ROTATION_R) }
    }
    pub fn read_total_rotation_left(&self) -> i32 {
        unsafe { self.uio_acc.read_memi32(TOTAL_ROTATION_L) }
    }
    pub fn get_total_rotation_right(&self) -> f32 {
        (self.read_total_rotation_right() as f32) / 360.
    }
    pub fn get_total_rotation_left(&self) -> f32 {
        (self.read_total_rotation_left() as f32) / 360.
    }
    pub fn get_max_accel(&self) -> i32 {
        self.accel_max
    }
    pub fn get_max_rpm(&self) -> f32 {
        (self.accel_max as f32) * 60. / self.fb_edge_period / 360.
    }
}
