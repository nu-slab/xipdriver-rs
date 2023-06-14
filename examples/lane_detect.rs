
use std::{thread, time};

fn main() {
    let map = xipdriver_rs::hwh_parser::parse("fc_design.hwh").unwrap();

    let mut ld = xipdriver_rs::umv_lane_detector::UmvLaneDetector::new(
        &map["/umv_lane_detector_0"],
        "umv_lane_detector",
        "udmabuf_umv_ld",
    )
    .unwrap();

    let mut vfb_r = xipdriver_rs::v_frmbuf::VideoFrameBufRead::new(
        &map["/v_frmbuf_rd_0"],
        "v_frmbuf_rd",
        "udmabuf_vfbr",
    )
    .unwrap();

    let mut vfb_w = xipdriver_rs::v_frmbuf::VideoFrameBufWrite::new(
        &map["/v_frmbuf_wr_0"],
        "v_frmbuf_wr",
        "udmabuf_vfbw",
    )
    .unwrap();

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

    ld.video_mode = 0;
    ld.configure_all().unwrap();

    // start IP
    vfb_r.start();
    vfb_w.start();


    let img = image::open("examples/road.png").unwrap();
    let img_rgb = img.to_rgb8();
    let frame = img_rgb.to_vec();
    println!("write frame");
    vfb_r.write_frame(frame.as_ptr());
    thread::sleep(time::Duration::from_millis(100));

    println!("Lane detection");
    ld.start().unwrap();
    let points = ld.read_data();
    for p in points.iter() {
        println!("{}", p);
    }

    let rgb_frame = vfb_w.read_frame_as_image();
    rgb_frame.save("out.bmp").unwrap();

}
