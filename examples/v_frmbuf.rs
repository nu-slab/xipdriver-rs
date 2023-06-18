use std::time::Instant;
use std::{thread, time};
use xipdriver_rs::v_frmbuf::{VideoFrameBufRead, VideoFrameBufWrite};
use anyhow::Result;

fn main() -> Result<()> {
    let hw_json = xipdriver_rs::hwinfo::read("hwinfo.json")?;

    let mut vfb_r = VideoFrameBufRead::new(&hw_json["/v_frmbuf_rd_0"])?;

    let mut vfb_w = VideoFrameBufWrite::new(&hw_json["/v_frmbuf_wr_0"])?;


    let frame_width = 1280;
    let frame_height = 720;

    // v_frmbuf_read config
    vfb_r.frame_width = frame_width;
    vfb_r.frame_height = frame_height;
    vfb_r.set_format("RGB8")?;

    // v_frmbuf_write config
    vfb_w.frame_width = frame_width;
    vfb_w.frame_height = frame_height;
    vfb_w.set_format("RGB8")?;

    // start IP
    vfb_r.start()?;
    vfb_w.start()?;

    println!("Write & Read frames");
    let mut read_frames = Vec::new();
    for i in 0..10 {
        let frame: Vec<u8> = vec![0xFF / 9 * (9 - i); (frame_width * frame_height * 3) as usize];

        // Write to v_frmbuf_read

        let start = Instant::now();
        vfb_r.write_frame(frame.as_ptr())?;
        let end = start.elapsed();
        println!("PS->PL Write time:{:03}ms", end.as_secs_f64() * 1000.0);
        thread::sleep(time::Duration::from_millis(70));
        // Read from v_frmbuf_write
        let start2 = Instant::now();
        let rgb_frame = vfb_w.read_frame_as_image()?;
        let end2 = start2.elapsed();
        println!("PL->PS Read time:{:03}ms", end2.as_secs_f64() * 1000.0);
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
        read_frames[i].save(format!("out{}.bmp", i))?;
    }

    println!("Done!");

    vfb_r.stop();
    vfb_w.stop();
    Ok(())
}
