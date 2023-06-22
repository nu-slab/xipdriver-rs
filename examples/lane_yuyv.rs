use imageproc::drawing;
use image::Pixel;
use anyhow::Result;
use std::{thread, time};
use std::time::Instant;

use xipdriver_rs::v_frmbuf::{VideoFrameBufRead, VideoFrameBufWrite};
use xipdriver_rs::v_proc_ss::VideoProcSubsystemCsc;
use xipdriver_rs::umv_lane_detector::UmvLaneDetector;


fn main() -> Result<()> {
    let hw_json = xipdriver_rs::hwinfo::read("hwinfo.json")?;


    let mut vfb_r0 = VideoFrameBufRead::new(&hw_json["/yuyv2rgb/v_frmbuf_rd_0"])?;
    let mut vpss_csc = VideoProcSubsystemCsc::new(&hw_json["/yuyv2rgb/v_proc_ss_0"])?;
    let mut vfb_w0 = VideoFrameBufWrite::new(&hw_json["/yuyv2rgb/v_frmbuf_wr_0"])?;

    let mut vfb_r1 = VideoFrameBufRead::new(&hw_json["/lane_detection/v_frmbuf_rd_1"])?;
    let mut ld = UmvLaneDetector::new(&hw_json["/lane_detection/umv_lane_detector_0"])?;
    let mut vfb_w1 = VideoFrameBufWrite::new(&hw_json["/lane_detection/v_frmbuf_wr_1"])?;

    let frame_width = ld.get_image_width();
    let frame_height = ld.get_image_height();

    // v_frmbuf_read config
    vfb_r0.frame_width = frame_width;
    vfb_r0.frame_height = frame_height;
    vfb_r0.set_format("YUYV")?;

    // // v_proc_ss config
    vpss_csc.frame_width = frame_width;
    vpss_csc.frame_height = frame_height;
    vpss_csc.set_format("4:2:2", "RGB");

    // v_frmbuf_write config
    vfb_w0.frame_width = frame_width;
    vfb_w0.frame_height = frame_height;
    vfb_w0.set_format("RGB8")?;

    // v_frmbuf_read config
    vfb_r1.frame_width = frame_width;
    vfb_r1.frame_height = frame_height;
    vfb_r1.set_format("RGB8")?;
    vfb_r1.tie(&vfb_w0);

    ld.video_mode = 0;
    ld.configure_all()?;

    // v_frmbuf_write config
    vfb_w1.frame_width = frame_width;
    vfb_w1.frame_height = frame_height;
    vfb_w1.set_format("RGB8")?;

    // start IP
    vfb_r0.start()?;
    vfb_w0.start()?;
    vpss_csc.start()?;
    vfb_r1.start()?;
    vfb_w1.start()?;


    let frames = 10;

    let img = image::open("examples/road.png")?;
    let img_rgb = img.to_rgb8();
    let in_rgb_frame = img_rgb.to_vec();
    let frame = rgb2yuyv(&in_rgb_frame);

    for i in 0..frames {
        println!("YUYV -> RGB: {}", i);
        let total_start = Instant::now();
        vfb_r0.write_frame(frame.as_ptr())?;
        let end = total_start.elapsed();
        println!("  PS->PL Write time:{:.02}ms", end.as_secs_f64() * 1000.0);

        // let start = Instant::now();
        // let rgb_frame = vfb_w0.read_frame()?;
        // // let rgb_frame = yuyv2rgb(frame);
        // let end = start.elapsed();
        // println!("  PL->PS Read time:{:.02}ms", end.as_secs_f64() * 1000.0);

        // println!("RGB -> Lane Detection: {}", i);
        // let start = Instant::now();
        // vfb_r1.write_frame(rgb_frame.as_ptr())?;
        // let end = start.elapsed();
        // println!("  PS->PL Write time:{:.02}ms", end.as_secs_f64() * 1000.0);

        thread::sleep(time::Duration::from_micros(300));

        let start = Instant::now();
        ld.start()?;
        let points = ld.read_data();
        let end = start.elapsed();
        let p_end = total_start.elapsed();
        println!("  Lane detect Read time:{:.02}ms", end.as_secs_f64() * 1000.0);
        println!("Total processing time:{:.02}ms, {:.02}FPS", p_end.as_secs_f64() * 1000.0, 1./p_end.as_secs_f64());

        let start = Instant::now();
        let mut lane_image = vfb_w1.read_frame_as_image()?;
        let end = start.elapsed();
        let total_end = total_start.elapsed();
        println!("  PL->PS Read time:{:.02}ms", end.as_secs_f64() * 1000.0);
        println!("Total read time:{:.02}ms, {:.02}FPS", total_end.as_secs_f64() * 1000.0, 1./total_end.as_secs_f64());

        println!("Points: {}", points.len());
        for p in points.iter() {
            let color = if p.direction == 1 { [255, 0, 0] } else { [0, 255, 0] };
            drawing::draw_filled_circle_mut(&mut lane_image, (p.x as i32, p.y as i32), 3, *image::Rgb::from_slice(&color));
        }
        lane_image.save(format!("lane{}.jpg", i))?;
        println!("");
    }
    vfb_r0.stop();
    vfb_w0.stop();
    vpss_csc.stop();
    ld.stop();
    vfb_r1.stop();
    vfb_w1.stop();
    Ok(())
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
