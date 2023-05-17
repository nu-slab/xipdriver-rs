fn main() {
    let map = xipdriver_rs::hwh_parser::parse("fc_design.hwh").unwrap();

    let vdma = xipdriver_rs::vdma::AxiVdma::new(&map["/axi_vdma_0"], "dma", "udmabuf0").unwrap();
    let gpio = xipdriver_rs::axigpio::AxiGpio::new(&map["/axi_gpio_0"], "gpio").unwrap();

    println!("is_running: {}", vdma.is_running());
    vdma.start();
    println!("is_running: {}", vdma.is_running());

    println!("{}", gpio.read_data(1).unwrap());
    println!("{}", gpio.read_data(2).unwrap());
    gpio.write_data(2, 123456).unwrap();
    println!("{}", gpio.read_data(2).unwrap());

    vdma.reset();
}
