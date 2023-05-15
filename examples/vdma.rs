

fn main() {
    let map = xipdriver_rs::hwh_parser::parse("fc_design.hwh").unwrap();
    //let uio = xipdriver_rs::mem::new("axi_vdma").unwrap();
    let a = xipdriver_rs::vdma::AxiVdma::new(&map["/axi_vdma_0"]).unwrap();

    println!("{:?}", a.is_running());

}
