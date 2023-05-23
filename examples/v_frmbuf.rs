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
    let gpio = xipdriver_rs::axigpio::AxiGpio::new(&map["/axi_gpio_0"], "gpio").unwrap();

    println!("{:06x}", gpio.read_data(1).unwrap());

    let frame_width = 1280;
    let frame_height = 720;

    // v_frmbuf config
    vfb_r.image_width = frame_width;
    vfb_r.image_height = frame_height;
    vfb_r.set_format("YUYV");
    vfb_r.set_auto_restart_enable(true);
    vfb_r.write_format();
    vfb_r.set_framebuf_addr();

    // v_frmbuf write test
    let frame: [u8; 1280 * 720 * 2] = [0x40; 1280 * 720 * 2];
    vfb_r.write_frame(&frame);
    println!("vfb_r.is_ready: {}", vfb_r.is_ready());
    vfb_r.start();
    println!("vfb_r.is_ready: {}", vfb_r.is_ready());

    // v_proc config
    csc.set_frame_size(frame_width, frame_height);
    csc.set_mode(2, 0).unwrap();
    csc.set_auto_restart_enable(true);

    println!("csc.is_ready: {}", csc.is_ready());
    csc.start().unwrap();
    println!("csc.is_ready: {}", csc.is_ready());
    let csc_mat = csc.read_csc_matrix();
    for c in csc_mat.iter() {
        println!("{} ", c);
    }

    // read pixel data
    for _ in 1..100 {
        println!("{:06x}", gpio.read_data(1).unwrap());
    }
    vfb_r.set_auto_restart_enable(false);
    csc.set_auto_restart_enable(false);
}
