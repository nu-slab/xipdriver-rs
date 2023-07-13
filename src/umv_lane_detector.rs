use anyhow::{ensure, Result, Context, bail};

#[cfg(all(target_os = "linux", target_arch = "aarch64"))]
use jelly_mem_access::*;

use crate::json_as_map;
use crate::json_as_str;
use crate::json_as_u32;

const FINDLINES_STATUS:usize          = 0x00;
const FINDLINES_START:usize           = 0x04;
const FILTER_TYPE:usize               = 0x08;
const BIN_FILTER_THRESH:usize         = 0x0C;
const EDGE_FILTER_THRESH:usize        = 0x10;
const EDGE_SELECT_THRESH:usize        = 0x14;
const FINDLINES_VLINE_WIDTH_INTERVAL:usize  = 0x18;
const FINDLINES_VLINE_WIDTH_MIN:usize  = 0x1C;
const FINDLINES_VLINE_THRESH_INTERVAL:usize = 0x20;
const FINDLINES_DETECT_INTERVAL:usize = 0x24;
const MEM_BASE_ADDR:usize             = 0x28;
const MEM_SIZE:usize                  = 0x2C;
const FINDLINES_DETECT_COUNT:usize    = 0x30;
const VID_MODE:usize                  = 0x34;
const FL_HLINE_DIN_MASK:usize         = 0x38;
const FL_VLINE_DIN_MASK:usize         = 0x3C;
const FINDLINES_HLINE_WIDTH_INTERVAL:usize  = 0x40;
const FINDLINES_HLINE_WIDTH_MIN:usize  = 0x44;
const FINDLINES_HLINE_THRESH_INTERVAL:usize = 0x48;
const FL_VLINE_WIDTH_DETECT_MIN:usize = 0x4C;
const FL_HLINE_WIDTH_DETECT_MIN:usize = 0x50;
const FINDLINES_HORIZON:usize = 0x54;
const FL_SEQUENCE_RANGE:usize = 0x58;

#[derive(Debug, Clone, Copy)]
pub struct LanePoint {
    pub direction: u32,
    pub x: u32,
    pub y: u32,
}

impl std::fmt::Display for LanePoint {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "({}, {}): {}", self.x, self.y, self.direction)
    }
}


pub struct UmvLaneDetector {
    uio_acc: UioAccessor<usize>,
    udmabuf_acc: UdmabufAccessor<usize>,
    image_width: u32,
    image_height: u32,
    max_detect_interval: u32,
    pub filter_type: u32,
    pub bin_filter_thresh:  u32,
    pub edge_filter_thresh: u32,
    pub edge_select_thresh: u32,
    pub fl_vline_width_max: u32,
    pub fl_vline_width_min: u32,
    pub fl_vline_thresh: u32,
    pub fl_hline_width_max: u32,
    pub fl_hline_width_min: u32,
    pub fl_hline_thresh: u32,
    pub fl_detect_interval: u32,
    pub video_mode: u32,
    pub fl_hline_din_mask: u32,
    pub fl_vline_din_mask: u32,
    pub fl_vline_width_detect_min: u32,
    pub fl_hline_width_detect_min: u32,
    pub findlines_horizon: u32,
    pub fl_sequence_range: u32,
}

impl UmvLaneDetector {
    pub fn new(hw_info: &serde_json::Value) -> Result<Self> {
        let hw_object = json_as_map!(hw_info);
        let hw_params = json_as_map!(hw_object["params"]);
        let vendor = json_as_str!(hw_object["vendor"]);
        let library = json_as_str!(hw_object["library"]);
        let name = json_as_str!(hw_object["name"]);
        let uio_name = json_as_str!(hw_object["uio"]);
        let udmabuf_name = json_as_str!(hw_object["udmabuf"][0]);
        let image_width = json_as_u32!(hw_params["IMAGE_WIDTH"]);
        let image_height = json_as_u32!(hw_params["IMAGE_HEIGHT"]);
        let max_detect_lines = json_as_u32!(hw_params["MAX_DETECT_LINES"]);
        let max_detect_interval = (max_detect_lines as f32).log2() as u32;
        let filter_type = json_as_u32!(hw_params["FILTER_TYPE_DEFAULT"]);
        ensure!(
            vendor == "slab" &&
            library == "umv_project" &&
            name == "umv_lane_detector",
            "UmvLaneDetector::new(): This IP is not supported. ({})",
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
        Ok(UmvLaneDetector {
            uio_acc: uio,
            udmabuf_acc: udmabuf,
            image_width,
            image_height,
            max_detect_interval,
            filter_type,
            bin_filter_thresh: 120,
            edge_filter_thresh: 85,
            edge_select_thresh: 3,
            fl_vline_width_max: 70,
            fl_vline_width_min: 70,
            fl_vline_thresh: 30,
            fl_hline_width_max: 65,
            fl_hline_width_min: 65,
            fl_hline_thresh: 15,
            fl_detect_interval: 6,
            video_mode: 0,
            fl_hline_din_mask: 0b0001,
            fl_vline_din_mask: 0b1110,
            fl_vline_width_detect_min: 10,
            fl_hline_width_detect_min: 10,
            findlines_horizon: 0,
            fl_sequence_range: 5,
        })
    }
    pub fn get_status(&self) -> u32 {
        unsafe { self.uio_acc.read_mem32(FINDLINES_STATUS) }
    }
    pub fn is_idle(&self) -> bool {
        self.get_status() == 0
    }
    pub fn is_waiting(&self) -> bool {
        self.get_status() == 1
    }
    pub fn is_running(&self) -> bool {
        self.get_status() == 2
    }
    pub fn is_done(&self) -> bool {
        self.get_status() == 3
    }
    pub fn start(&self) -> Result<()> {
        self.configure_all()?;
        unsafe {
            self.uio_acc.write_mem32(FINDLINES_START, 0x01);
        }
        Ok(())
    }
    pub fn stop(&self) {
        unsafe {
            self.uio_acc.write_mem32(FINDLINES_START, 0x00);
        }
        while self.get_status() % 3 != 0 { }
    }
    pub fn write_framebuf_addr(&self) {
        unsafe {
            self.uio_acc
                .write_mem32(MEM_BASE_ADDR, self.udmabuf_acc.phys_addr() as u32);
            self.uio_acc
                .write_mem32(MEM_SIZE, (self.udmabuf_acc.size() / 4) as u32);
        }
    }
    pub fn read_data(&self) -> Vec<LanePoint> {
        self.stop();
        let detect_cnt = unsafe { self.uio_acc.read_mem32(FINDLINES_DETECT_COUNT) } as usize;
        let data_num = detect_cnt.min(self.udmabuf_acc.size() / 4);
        let mut buf = Vec::with_capacity(data_num);
        for i in 0..data_num {
            let data = unsafe { self.udmabuf_acc.read_mem32(0x00 + 4 * i) };
            let point = LanePoint {
                direction: (data >> 28) & 0xf,
                x: (data >> 14) & 0x3fff,
                y: data & 0x3fff,
            };
            buf.push(point);
        }
        buf
    }
    pub fn configure_all(&self) -> Result<()> {
        self.write_filter_type();
        self.write_bin_filter_thresh();
        self.write_edge_filter_thresh();
        self.write_edge_select_thresh();
        self.write_fl_vline_width()?;
        self.write_fl_vline_thresh();
        self.write_fl_hline_width()?;
        self.write_fl_hline_thresh();
        self.write_fl_detect_interval();
        self.write_vid_mode();
        self.write_fl_hline_din_mask();
        self.write_fl_vline_din_mask();
        self.write_framebuf_addr();
        self.write_fl_vline_width_detect_min();
        self.write_fl_hline_width_detect_min();
        self.write_findlines_horizon()?;
        self.write_fl_sequence_range();
        Ok(())
    }
    pub fn write_filter_type(&self) {
        unsafe {
            self.uio_acc.write_mem32(FILTER_TYPE, self.filter_type);
        }
    }
    pub fn write_bin_filter_thresh(&self) {
        unsafe {
            self.uio_acc.write_mem32(BIN_FILTER_THRESH, self.bin_filter_thresh);
        }
    }
    pub fn write_edge_filter_thresh(&self) {
        unsafe {
            self.uio_acc.write_mem32(EDGE_FILTER_THRESH, self.edge_filter_thresh);
        }
    }
    pub fn write_edge_select_thresh(&self) {
        unsafe {
            self.uio_acc.write_mem32(EDGE_SELECT_THRESH, self.edge_select_thresh);
        }
    }
    pub fn write_fl_vline_width_inc_interval(&self, val: u32) {
        unsafe {
            self.uio_acc.write_mem32(FINDLINES_VLINE_WIDTH_INTERVAL, val);
        }
    }
    pub fn write_fl_vline_width_min(&self) {
        unsafe {
            self.uio_acc.write_mem32(FINDLINES_VLINE_WIDTH_MIN, self.fl_vline_width_min);
        }
    }
    pub fn write_fl_vline_width(&self) -> Result<()> {
        ensure!(self.findlines_horizon <= self.image_height, "image_height < findlines_horizon");
        ensure!(self.fl_vline_width_min <= self.fl_vline_width_max, "fl_width_max < fl_width_min");
        let interval = (self.image_height - self.findlines_horizon) / (self.fl_vline_width_max - self.fl_vline_width_min + 1);
        self.write_fl_vline_width_inc_interval(interval);
        self.write_fl_vline_width_min();
        Ok(())
    }
    pub fn write_fl_vline_thresh(&self) {
        // let interval = self.image_height / (self.fl_vline_thresh_max + 1);
        unsafe {
            self.uio_acc.write_mem32(FINDLINES_VLINE_THRESH_INTERVAL, self.fl_vline_thresh);
        }
    }
    pub fn write_fl_hline_width_inc_interval(&self, val: u32) {
        unsafe {
            self.uio_acc.write_mem32(FINDLINES_HLINE_WIDTH_INTERVAL, val);
        }
    }
    pub fn write_fl_hline_width_min(&self) {
        unsafe {
            self.uio_acc.write_mem32(FINDLINES_HLINE_WIDTH_MIN, self.fl_hline_width_min);
        }
    }
    pub fn write_fl_hline_width(&self) -> Result<()> {
        ensure!(self.findlines_horizon <= self.image_height, "image_height < findlines_horizon");
        ensure!(self.fl_hline_width_min <= self.fl_hline_width_max, "fl_width_max < fl_width_min");
        let interval = (self.image_height - self.findlines_horizon) / (self.fl_hline_width_max - self.fl_hline_width_min + 1);
        self.write_fl_hline_width_inc_interval(interval);
        self.write_fl_hline_width_min();
        Ok(())
    }
    pub fn write_fl_hline_thresh(&self) {
        // let interval = self.image_height / (self.fl_hline_thresh_max + 1);
        unsafe {
            self.uio_acc.write_mem32(FINDLINES_HLINE_THRESH_INTERVAL, self.fl_hline_thresh);
        }
    }
    pub fn write_fl_detect_interval(&self) {
        let interval_min = self.fl_detect_interval.min(self.max_detect_interval);
        unsafe {
            self.uio_acc.write_mem32(FINDLINES_DETECT_INTERVAL, interval_min);
        }
    }
    pub fn write_vid_mode(&self) {
        unsafe {
            self.uio_acc.write_mem32(VID_MODE, self.video_mode);
        }
    }
    pub fn write_fl_hline_din_mask(&self) {
        unsafe {
            self.uio_acc.write_mem32(FL_HLINE_DIN_MASK, self.fl_hline_din_mask);
        }
    }
    pub fn write_fl_vline_din_mask(&self) {
        unsafe {
            self.uio_acc.write_mem32(FL_VLINE_DIN_MASK, self.fl_vline_din_mask);
        }
    }

    pub fn write_fl_vline_width_detect_min(&self) {
        unsafe {
            self.uio_acc.write_mem32(FL_VLINE_WIDTH_DETECT_MIN, self.fl_vline_width_detect_min);
        }
    }
    pub fn write_fl_hline_width_detect_min(&self) {
        unsafe {
            self.uio_acc.write_mem32(FL_HLINE_WIDTH_DETECT_MIN, self.fl_hline_width_detect_min);
        }
    }
    pub fn write_findlines_horizon(&self) -> Result<()> {
        ensure!(self.findlines_horizon <= self.image_height, "image_height < findlines_horizon");
        unsafe {
            self.uio_acc.write_mem32(FINDLINES_HORIZON, self.findlines_horizon);
        }
        Ok(())
    }
    pub fn write_fl_sequence_range(&self) {
        unsafe {
            self.uio_acc.write_mem32(FL_SEQUENCE_RANGE, self.fl_sequence_range);
        }
    }
    pub fn read_params(&mut self) {
        unsafe {
            self.filter_type        = self.uio_acc.read_mem32(FILTER_TYPE);
            self.bin_filter_thresh  = self.uio_acc.read_mem32(BIN_FILTER_THRESH);
            self.edge_filter_thresh = self.uio_acc.read_mem32(EDGE_FILTER_THRESH);
            self.edge_select_thresh = self.uio_acc.read_mem32(EDGE_SELECT_THRESH);
            let fl_vline_width_interval = self.uio_acc.read_mem32(FINDLINES_VLINE_WIDTH_INTERVAL);
            self.fl_vline_width_min = self.uio_acc.read_mem32(FINDLINES_VLINE_WIDTH_MIN);
            self.findlines_horizon = self.uio_acc.read_mem32(FINDLINES_HORIZON);
            self.fl_vline_width_max = ((self.image_height - self.findlines_horizon) / fl_vline_width_interval) + self.fl_vline_width_min - 1;
            let fl_vline_thresh_interval = self.uio_acc.read_mem32(FINDLINES_VLINE_THRESH_INTERVAL);
            self.fl_vline_thresh = fl_vline_thresh_interval; // (self.image_height / fl_vline_thresh_interval) - 1;
            let fl_hline_width_interval = self.uio_acc.read_mem32(FINDLINES_HLINE_WIDTH_INTERVAL);
            self.fl_hline_width_min = self.uio_acc.read_mem32(FINDLINES_HLINE_WIDTH_MIN);
            self.fl_hline_width_max = ((self.image_height - self.findlines_horizon) / fl_hline_width_interval) + self.fl_hline_width_min - 1;
            let fl_hline_thresh_interval = self.uio_acc.read_mem32(FINDLINES_HLINE_THRESH_INTERVAL);
            self.fl_hline_thresh = fl_hline_thresh_interval; //(self.image_height / fl_hline_thresh_interval) - 1;
            self.fl_detect_interval = self.uio_acc.read_mem32(FINDLINES_DETECT_INTERVAL);
            self.video_mode = self.uio_acc.read_mem32(VID_MODE);
            self.fl_hline_din_mask = self.uio_acc.read_mem32(FL_HLINE_DIN_MASK);
            self.fl_vline_din_mask = self.uio_acc.read_mem32(FL_VLINE_DIN_MASK);
            self.fl_sequence_range = self.uio_acc.read_mem32(FL_SEQUENCE_RANGE);
        }
    }
    pub fn get_image_width(&self) -> u32 {
        self.image_width
    }

    pub fn get_image_height(&self) -> u32 {
        self.image_height
    }

}
