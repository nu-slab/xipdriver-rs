use xipdriver_rs::v_frmbuf::VideoFrameBufWrite;
use xipdriver_rs::vdma::AxiVdmaMM2S;
use std::{thread, time};

fn main() {
    let hw_json = xipdriver_rs::hwinfo::read("hwinfo.json").unwrap();

    let mut vdma = AxiVdmaMM2S::new(&hw_json["/axi_vdma_0"]).unwrap();

    let mut vfb_w = VideoFrameBufWrite::new(&hw_json["/v_frmbuf_wr_0"]).unwrap();

    let frame_width = 1280;
    let frame_height = 720;

    // vdma config
    vdma.frame_width = frame_width;
    vdma.frame_height = frame_height;
    vdma.bytes_per_pix = 3;
    vdma.pix_per_clk = 1;

    // v_frmbuf_write config
    vfb_w.frame_width = frame_width;
    vfb_w.frame_height = frame_height;
    vfb_w.set_format("RGB8");

    // start IP
    vfb_w.start();
    println!("is_running: {}", vdma.is_running());
    vdma.start().unwrap();
    println!("is_running: {}", vdma.is_running());

    for i in 0..10 {
        let frame: Vec<u8> = vec![0xFF / 9 * (9 - i); (frame_width * frame_height * 3) as usize];
        vdma.write_frame(frame.as_ptr()).unwrap();
        let start = time::Instant::now();
        let rgb_frame = vfb_w.read_frame_as_image();
        let end = start.elapsed();
        println!("{}秒経過しました。", end.as_secs_f32());
        rgb_frame.save(format!("out{}.bmp", i)).unwrap();
    }

    for _ in 0..100 {
        println!("{}", vdma.read_active_frame());
        thread::sleep(time::Duration::from_millis(10));
    }

    vdma.reset();
    vfb_w.stop();
}
