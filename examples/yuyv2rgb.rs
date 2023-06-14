use xipdriver_rs::v_frmbuf::{VideoFrameBufRead, VideoFrameBufWrite};
use xipdriver_rs::v_proc_ss::VideoProcSubsystemCsc;
use std::time::Instant;

fn main() {
    let hw_json = xipdriver_rs::hwinfo::read("hwinfo.json").unwrap();

    let mut vfb_r = VideoFrameBufRead::new(&hw_json["/v_frmbuf_rd_0"]).unwrap();

    let mut vpss_csc = VideoProcSubsystemCsc::new(&hw_json["/v_proc_ss_0"]).unwrap();

    let mut vfb_w = VideoFrameBufWrite::new(&hw_json["/v_frmbuf_wr_0"]).unwrap();

    let frame_width = 1280;
    let frame_height = 720;

    // v_frmbuf_read config
    vfb_r.frame_width = frame_width;
    vfb_r.frame_height = frame_height;
    vfb_r.set_format("YUYV");

    // v_proc_ss config
    vpss_csc.frame_width = frame_width;
    vpss_csc.frame_height = frame_height;
    vpss_csc.set_format("4:2:2", "RGB");

    // v_frmbuf_write config
    vfb_w.frame_width = frame_width;
    vfb_w.frame_height = frame_height;
    vfb_w.set_format("RGB8");

    // start IP
    vpss_csc.start().unwrap();
    vfb_w.start();

    for k in vpss_csc.read_csc_matrix().iter() {
        print!("{} ", k);
    }
    println!();

    println!("Write & Read frames");
    let mut read_frames = Vec::new();
    for i in 0..10 {
        let frame: Vec<u8> = vec![0xFF / 9 * (9 - i); (frame_width * frame_height * 3) as usize];
        let frame_yuyv = rgb2yuyv(&frame);

        // Write to v_frmbuf_read
        let start = Instant::now();
        vfb_r.write_frame(frame_yuyv.as_ptr());
        let end = start.elapsed();
        println!("Write: {} msec", end.as_secs_f32() * 1000.);

        // Read from v_frmbuf_write
        let start = Instant::now();
        let rgb_frame = vfb_w.read_frame_as_image();
        let end = start.elapsed();
        println!("Read: {} msec", end.as_secs_f32() * 1000.);

        println!(
            "Pixel(0, 0): WriteRGB: [{}, {}, {}], WriteYUYV: [{}, {}], Read: {:?}",
            frame[0],
            frame[1],
            frame[2],
            frame_yuyv[0],
            frame_yuyv[1],
            rgb_frame.get_pixel(0, 0)
        );
        read_frames.push(rgb_frame);
    }

    println!("Save frames");
    for i in 0..read_frames.len() {
        println!("Writing out{}.bmp...", i);
        read_frames[i].save(format!("out{}.bmp", i)).unwrap();
    }

    println!("Done!");

    vfb_r.stop();
    vpss_csc.stop();
    vfb_w.stop();
}


fn rgb2yuyv(rgb: &[u8]) -> Vec<u8> {
    assert_eq!(rgb.len() % 3, 0);
    let length = rgb.len() / 3;
    let mut yuyv = vec![0; length * 2];
    for i in 0..length {
        let r = rgb[3 * i + 0] as f64;
        let g = rgb[3 * i + 1] as f64;
        let b = rgb[3 * i + 2] as f64;
        let y = 0.257 * r + 0.504 * g + 0.098 * b + 16.0;
        let u = -0.148 * r - 0.291 * g + 0.439 * b + 128.0;
        let v = 0.439 * r - 0.368 * g - 0.071 * b + 128.0;
        if i % 2 == 0 {
            yuyv[2 * i + 0] = y.max(0.0).min(255.0) as u8;
            yuyv[2 * i + 1] = u.max(0.0).min(255.0) as u8;
        } else {
            yuyv[2 * i + 0] = y.max(0.0).min(255.0) as u8;
            yuyv[2 * i + 1] = v.max(0.0).min(255.0) as u8;
        }
    }
    yuyv
}
