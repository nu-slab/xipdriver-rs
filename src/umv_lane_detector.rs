use anyhow::{ensure, Result, Context, bail};

#[cfg(all(target_os = "linux", target_arch = "aarch64"))]
use jelly_mem_access::*;


const FINDLINES_STATUS:usize          = 0x00;
const FINDLINES_START:usize           = 0x04;
const FILTER_TYPE:usize               = 0x08;
const BIN_FILTER_THRESH:usize         = 0x0C;
const EDGE_FILTER_THRESH:usize        = 0x10;
const EDGE_SELECT_THRESH:usize        = 0x14;
const FINDLINES_LINE_WIDTH_INTERVAL:usize  = 0x18;
const FINDLINES_LINE_WIDTH_MIN:usize  = 0x1C;
const FINDLINES_THRESH:usize          = 0x20;
const FINDLINES_DETECT_INTERVAL:usize = 0x24;
const MEM_BASE_ADDR:usize             = 0x28;
const MEM_SIZE:usize                  = 0x2C;
const FINDLINES_DETECT_COUNT:usize    = 0x30;
const VID_MODE:usize                  = 0x34;

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
    max_detect_lines: u32,
    pub filter_type: u32,
    pub bin_filter_thresh:  u32,
    pub edge_filter_thresh: u32,
    pub edge_select_thresh: u32,
    pub fl_width_max: u32,
    pub fl_width_min: u32,
    pub fl_thresh: u32,
    pub fl_detect_interval: u32,
    pub video_mode: u32,
}

impl UmvLaneDetector {
    pub fn new(hw_info: &serde_json::Value) -> Result<Self> {
        let hw_object = hw_info.as_object().context("hw_object is not an object type")?;
        let hw_params = hw_object["params"].as_object().context("hw_params is not an object type")?;
        let vendor = hw_object["vendor"].as_str().context("vendor is not string")?;
        let library = hw_object["library"].as_str().context("library is not string")?;
        let name = hw_object["name"].as_str().context("name is not string")?;
        let uio_name = hw_object["uio"].as_str().context("uio_name is not string")?;
        let udmabuf_name = hw_object["udmabuf"][0].as_str().context("udmabuf_name is not string")?;
        let width = hw_params["VID_H_ACTIVE"].as_str().context("VID_H_ACTIVE is not string")?.parse().context("Cannot convert VID_H_ACTIVE to numeric")?;
        let height = hw_params["VID_V_ACTIVE"].as_str().context("VID_V_ACTIVE is not string")?.parse().context("Cannot convert VID_V_ACTIVE to numeric")?;
        let max_detect_lines = hw_params["MAX_DETECT_LINES"].as_str().context("MAX_DETECT_LINES is not string")?.parse().context("Cannot convert MAX_DETECT_LINES to numeric")?;
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
            image_width: width,
            image_height: height,
            max_detect_lines: max_detect_lines,
            filter_type: 1,
            bin_filter_thresh: 120,
            edge_filter_thresh: 85,
            edge_select_thresh: 3,
            fl_width_max: 68,
            fl_width_min: 64,
            fl_thresh: 20,
            fl_detect_interval: 6,
            video_mode: 0,
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
                .write_mem32(MEM_SIZE, self.udmabuf_acc.size() as u32);
        }
    }
    pub fn read_data(&self) -> Vec<LanePoint> {
        self.stop();
        let data_num = unsafe { self.uio_acc.read_mem32(FINDLINES_DETECT_COUNT) } as usize;
        println!("data_num: {}", data_num);
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
        self.write_fl_width()?;
        self.write_fl_width_min();
        self.write_fl_thresh();
        self.write_fl_detect_interval();
        self.write_vid_mode();
        self.write_framebuf_addr();
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
    pub fn write_fl_width_inc_interval(&self, val: u32) {
        unsafe {
            self.uio_acc.write_mem32(FINDLINES_LINE_WIDTH_INTERVAL, val);
        }
    }
    pub fn write_fl_width_min(&self) {
        unsafe {
            self.uio_acc.write_mem32(FINDLINES_LINE_WIDTH_MIN, self.fl_width_min);
        }
    }
    pub fn write_fl_width(&self) -> Result<()> {
        ensure!(self.fl_width_min <= self.fl_width_max, "fl_width_max < fl_width_min");
        let interval = self.image_height / (self.fl_width_max - self.fl_width_min + 1);
        self.write_fl_width_inc_interval(interval);
        self.write_fl_width_min();
        Ok(())
    }
    pub fn write_fl_thresh(&self) {
        unsafe {
            self.uio_acc.write_mem32(FINDLINES_THRESH, self.fl_thresh);
        }
    }
    pub fn write_fl_detect_interval(&self) {
        unsafe {
            self.uio_acc.write_mem32(FINDLINES_DETECT_INTERVAL, self.fl_detect_interval);
        }
    }
    pub fn write_vid_mode(&self) {
        unsafe {
            self.uio_acc.write_mem32(VID_MODE, self.video_mode);
        }
    }

}
