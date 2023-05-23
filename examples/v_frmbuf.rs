fn main() {
    let map = xipdriver_rs::hwh_parser::parse("fc_design.hwh").unwrap();

    let mut vfb_r = xipdriver_rs::v_frmbuf::VideoFrameBufRead::new(
        &map["/v_frmbuf_rd_0"],
        "v_frmbuf_rd",
        "udmabuf_vfbr",
    )
    .unwrap();

    let mut csc =
        xipdriver_rs::v_proc_ss::VideoProcSubsystemCsc::new(&map["/v_proc_ss_0"], "v_proc_ss")
            .unwrap();

    let mut vfb_w = xipdriver_rs::v_frmbuf::VideoFrameBufWrite::new(
        &map["/v_frmbuf_wr_0"],
        "v_frmbuf_wr",
        "udmabuf_vfbw",
    )
    .unwrap();

    let gpio = xipdriver_rs::axigpio::AxiGpio::new(&map["/axi_gpio_0"], "gpio").unwrap();

    println!("{:06x}", gpio.read_data(1).unwrap());

    let frame_width = 1280;
    let frame_height = 720;

    // v_frmbuf_read config
    vfb_r.image_width = frame_width;
    vfb_r.image_height = frame_height;
    vfb_r.set_format("YUYV");
    vfb_r.set_auto_restart_enable(true);
    vfb_r.write_format();
    vfb_r.set_framebuf_addr();

    // v_proc config
    csc.set_frame_size(frame_width, frame_height);
    csc.set_mode(2, 0).unwrap();
    csc.set_auto_restart_enable(true);

    // v_frmbuf_write config
    vfb_w.image_width = frame_width;
    vfb_w.image_height = frame_height;
    vfb_w.set_format("RGB8");
    vfb_w.set_auto_restart_enable(true);
    vfb_w.write_format();
    vfb_w.set_framebuf_addr();

    // generate test frame
    let frame: Vec<u8> = vec![0xFF; 1280 * 720 * 3];
    let frame_yuyv = rgb2yuyv(&frame);
    for i in 0..10 {
        print!("{} ", frame_yuyv[i]);
    }
    println!();

    // write frame to v_frmbuf_read
    vfb_r.write_frame(&frame_yuyv);

    // start IPs
    vfb_r.start();
    csc.start().unwrap();
    vfb_w.start();

    println!("read");
    let rgb_frame = vfb_w.read_frame();
    println!("save");
    rgb_frame.save("out.png").unwrap();
    println!("done");

    vfb_r.set_auto_restart_enable(false);
    csc.set_auto_restart_enable(false);
    vfb_w.set_auto_restart_enable(false);
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
