use xipdriver_rs::axidma::AxiDma;
use anyhow::Result;

fn main() -> Result<()> {
    let hw_info = xipdriver_rs::hwinfo::read("/slab/hwinfo.json")?;
    let mut dma = AxiDma::new(&hw_info["/axi_dma_0"])?;
    let v = vec![1, 2, 3, 4, 5, 6, 7, 8];

    dma.start();
    println!("start");

    dma.write(&v)?;
    println!("write");

    let v2: Vec<i32> = dma.read(v.len())?;
    println!("read");

    dma.stop();
    println!("stop");

    println!("{:?}", v2);
    Ok(())
}
