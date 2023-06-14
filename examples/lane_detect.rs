use imageproc::drawing;
use image::Pixel;

use xipdriver_rs::umv_lane_detector::UmvLaneDetector;
use xipdriver_rs::v_frmbuf::{VideoFrameBufRead, VideoFrameBufWrite};

fn main() {
    let hw_json = xipdriver_rs::hwinfo::read("hwinfo.json").unwrap();

    let mut ld = UmvLaneDetector::new(&hw_json["/umv_lane_detector_0"]).unwrap();

    let mut vfb_r = VideoFrameBufRead::new(&hw_json["/v_frmbuf_rd_0"]).unwrap();

    let mut vfb_w = VideoFrameBufWrite::new(&hw_json["/v_frmbuf_wr_0"]).unwrap();

    let frame_width = 1280;
    let frame_height = 720;

    // v_frmbuf_read config
    vfb_r.frame_width = frame_width;
    vfb_r.frame_height = frame_height;
    vfb_r.set_format("RGB8");

    // v_frmbuf_write config
    vfb_w.frame_width = frame_width;
    vfb_w.frame_height = frame_height;
    vfb_w.set_format("RGB8");

    ld.bin_filter_thresh = 0;
    ld.configure_all().unwrap();

    // start IP
    vfb_r.start();
    vfb_w.start();


    let img = image::open("examples/road.png").unwrap();
    let img_rgb = img.to_rgb8();
    let frame = img_rgb.to_vec();
    println!("write frame");
    vfb_r.write_frame(frame.as_ptr());

    println!("Lane detection");
    ld.start().unwrap();
    let points = ld.read_data();

    let mut rgb_frame = vfb_w.read_frame_as_image();
    for p in points.iter() {
        println!("{}", p);
        let color = if p.direction == 1 { [255, 0, 0] } else { [0, 255, 0] };
        drawing::draw_filled_circle_mut(&mut rgb_frame, (p.x as i32, p.y as i32), 3, *image::Rgb::from_slice(&color));
    }
    rgb_frame.save("out.bmp").unwrap();

    vfb_r.stop();
    vfb_w.stop();

}
