fn main() {
    let map = xipdriver_rs::hwh_parser::parse("fc_design.hwh").unwrap();

    let mut vfb_r = xipdriver_rs::v_frmbuf::VideoFrameBufRead::new(&map["/v_frmbuf_rd_0"], "v_frmbuf_rd", "udmabuf_vfbr").unwrap();
    let gpio = xipdriver_rs::axigpio::AxiGpio::new(&map["/axi_gpio_0"], "gpio").unwrap();
    println!("{:06x}", gpio.read_data(1).unwrap());
    vfb_r.image_width = 1280;
    vfb_r.image_height = 720;
    vfb_r.set_format("RGB8");
    vfb_r.set_auto_restart_enable(true);
    vfb_r.write_format();
    vfb_r.set_framebuf_addr();
    let frame: [u8; 1280*720*3] = [0x64; 1280*720*3];
    vfb_r.write_frame(&frame);
    println!("{}",  vfb_r.is_ready());
    vfb_r.start();
    println!("{}",  vfb_r.is_ready());
    println!("{}", gpio.read_data(2).unwrap());
    for _ in 1..100 {
        println!("{:06x}", gpio.read_data(1).unwrap());
    }

}
