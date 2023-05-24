fn main() {
    let map = xipdriver_rs::hwh_parser::parse("fc_design.hwh").unwrap();

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

    // start IP
    vfb_w.start();

    println!("Write & Read frames");
    let mut read_frames = Vec::new();
    for i in 0..10 {
        let frame: Vec<u8> = vec![0xFF / 9 * (9 - i); (frame_width * frame_height * 3) as usize];

        // Write to v_frmbuf_read
        vfb_r.write_frame(frame.as_ptr());

        // Read from v_frmbuf_write
        let rgb_frame = vfb_w.read_frame();

        println!(
            "Pixel(0, 0): Write: [{}, {}, {}], Read: {:?}",
            frame[0],
            frame[1],
            frame[2],
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
    vfb_w.stop();
}
