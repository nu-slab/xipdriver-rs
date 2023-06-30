use imageproc::drawing;
use image::Pixel;

use xipdriver_rs::umv_lane_detector::UmvLaneDetector;
use xipdriver_rs::v_frmbuf::{VideoFrameBufRead, VideoFrameBufWrite};
use std::{thread, time};
use std::time::Instant;
use anyhow::Result;

fn main() -> Result<()> {
    let hw_json = xipdriver_rs::hwinfo::read("/umv/hwinfo.json")?;

    let mut ld = UmvLaneDetector::new(&hw_json["/lane_detection/umv_lane_detector"])?;

    let mut vfb_r = VideoFrameBufRead::new(&hw_json["/lane_detection/v_frmbuf_rd"])?;

    let mut vfb_w = VideoFrameBufWrite::new(&hw_json["/lane_detection/v_frmbuf_wr"])?;

    let frame_width = ld.get_image_width();
    let frame_height = ld.get_image_height();

    // v_frmbuf_read config
    vfb_r.frame_width = frame_width;
    vfb_r.frame_height = frame_height;
    vfb_r.set_format("RGB8")?;

    // v_frmbuf_write config
    vfb_w.frame_width = frame_width;
    vfb_w.frame_height = frame_height;
    vfb_w.set_format("RGB8")?;

    ld.configure_all()?;

    // start IP
    vfb_r.start()?;
    vfb_w.start()?;


    let img = image::open("examples/road.png")?;
    let img_rgb = img.to_rgb8();
    let frame = img_rgb.to_vec();

    for i in 0..9 {
        ld.video_mode = i;
        ld.configure_all()?;

        println!("write frame: {}", i);
        let start = Instant::now();
        vfb_r.write_frame(frame.as_ptr())?;
        let end = start.elapsed();
        println!("PS->PL Write time:{:03}ms", end.as_secs_f64() * 1000.0);

        thread::sleep(time::Duration::from_micros(300));

        let start = Instant::now();
        ld.start()?;
        let points = ld.read_data();
        let end = start.elapsed();
        println!("Lane detect Read time:{:03}ms", end.as_secs_f64() * 1000.0);

        let start = Instant::now();
        let mut rgb_frame = vfb_w.read_frame_as_image()?;
        let end = start.elapsed();
        println!("PL->PS Read time:{:03}ms", end.as_secs_f64() * 1000.0);

        println!("Points: {}", points.len());
        for p in points.iter() {
            let color = if p.direction == 1 { [255, 0, 0] } else { [0, 255, 0] };
            drawing::draw_filled_circle_mut(&mut rgb_frame, (p.x as i32, p.y as i32), 3, *image::Rgb::from_slice(&color));
        }
        rgb_frame.save(format!("out{}.bmp", i))?;
        println!("");
    }

    vfb_r.stop();
    vfb_w.stop();

    Ok(())
}
