fn main() {
    let map = xipdriver_rs::hwh_parser::parse("fc_design.hwh").unwrap();

    let a =
        xipdriver_rs::vdma::AxiVdma::new(&map["/axi_vdma_0"], "axi_vdma", "vdma_udmabuf0").unwrap();

    println!("is_running: {}", a.is_running());
    a.start();
    println!("is_running: {}", a.is_running());
    a.reset();
}
