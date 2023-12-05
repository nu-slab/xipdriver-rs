use anyhow::{bail, ensure, Context, Result};

use jelly_mem_access::*;

use crate::json_as_map;
use crate::json_as_str;
use crate::json_as_vec;

const DMACR: usize = 0x00;
const DMASR: usize = 0x04;
const SA: usize = 0x18;
const LENGTH: usize = 0x28;
const S2MM_OFFSET: usize = 0x30;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DmaChannelMode {
    MM2S,
    S2MM,
}

pub struct AxiDmaChannel {
    uio_acc: UioAccessor<usize>,
    udmabuf_acc: UdmabufAccessor<usize>,
    first_transfer: bool,
    mode: DmaChannelMode,
    offset: usize,
}

impl AxiDmaChannel {
    pub fn new(
        hw_info: &serde_json::Value,
        mode: DmaChannelMode,
        udmabuf_name: &str,
    ) -> Result<Self> {
        let hw_object = json_as_map!(hw_info);
        // let hw_params = json_as_map!(hw_object["params"]);
        let vendor = json_as_str!(hw_object["vendor"]);
        let library = json_as_str!(hw_object["library"]);
        let name = json_as_str!(hw_object["name"]);
        let uio_name = json_as_str!(hw_object["uio"]);
        ensure!(
            vendor == "xilinx.com" && library == "ip" && name == "axi_dma",
            "AxiDmaChannel::new(): This IP is not supported. ({})",
            name
        );
        let uio = match UioAccessor::<usize>::new_with_name(uio_name) {
            Ok(uio_acc) => uio_acc,
            Err(e) => {
                bail!("UioAccessor: {}", e)
            }
        };
        let udmabuf = match UdmabufAccessor::new(udmabuf_name, false) {
            Ok(udmabuf_acc) => udmabuf_acc,
            Err(e) => {
                bail!("UdmabufAccessor: {}", e)
            }
        };

        Ok(AxiDmaChannel {
            uio_acc: uio,
            udmabuf_acc: udmabuf,
            first_transfer: true,
            mode,
            offset: if mode == DmaChannelMode::MM2S {
                0
            } else {
                S2MM_OFFSET
            },
        })
    }

    pub fn is_running(&self) -> bool {
        unsafe { self.uio_acc.read_mem32(self.offset + DMASR) & 1 == 0 }
    }

    pub fn is_idle(&self) -> bool {
        unsafe { self.uio_acc.read_mem32(self.offset + DMASR) & 2 == 2 }
    }

    pub fn is_error(&self) -> bool {
        unsafe { self.uio_acc.read_mem32(self.offset + DMASR) & 0x70 != 0 }
    }

    fn write_buf_addr(&self) {
        let mem_phys_addr = self.udmabuf_acc.phys_addr() as u32;
        unsafe {
            self.uio_acc.write_mem32(self.offset + SA, mem_phys_addr);
        }
    }

    pub fn stop(&self) {
        unsafe {
            self.uio_acc.write_mem32(self.offset + DMACR, 0);
        }
        while self.is_running() {}
    }

    pub fn start(&mut self) {
        unsafe {
            self.uio_acc.write_mem32(self.offset + DMACR, 0x0001);
        }
        self.first_transfer = true;
    }
    pub fn write_len(&self, len: u32) {
        unsafe {
            self.uio_acc.write_mem32(self.offset + LENGTH, len);
        }
    }
    pub fn read_len(&self) -> u32 {
        unsafe { self.uio_acc.read_mem32(self.offset + LENGTH) }
    }
    pub fn write<V>(&mut self, data: &[V]) -> Result<()> {
        ensure!(
            self.mode == DmaChannelMode::MM2S,
            "Channel mode is not MM2S"
        );
        ensure!(self.is_running(), "DMA channel not started");
        ensure!(
            self.is_idle() || self.first_transfer,
            "DMA channel not idle"
        );
        let size = core::mem::size_of::<V>();
        unsafe {
            self.udmabuf_acc
                .copy_from(data.as_ptr(), 0, data.len() * size);
        }
        self.write_buf_addr();
        self.write_len((data.len() * size) as u32);
        self.first_transfer = false;
        Ok(())
    }
    pub fn write_with_size<V>(&mut self, data: &[V], size: usize) -> Result<()> {
        ensure!(
            self.mode == DmaChannelMode::MM2S,
            "Channel mode is not MM2S"
        );
        ensure!(self.is_running(), "DMA channel not started");
        ensure!(
            self.is_idle() || self.first_transfer,
            "DMA channel not idle"
        );
        ensure!(
            size <= data.len(),
            "The size of the transfer is too large. ({} > {})",
            size,
            data.len()
        );
        let size_of_v = core::mem::size_of::<V>();
        unsafe {
            self.udmabuf_acc
                .copy_from(data.as_ptr(), 0, size * size_of_v);
        }
        self.write_buf_addr();
        self.write_len((size * size_of_v) as u32);
        self.first_transfer = false;
        Ok(())
    }
    pub fn read<V>(&mut self, len: usize) -> Result<Vec<V>> {
        ensure!(
            self.mode == DmaChannelMode::S2MM,
            "Channel mode is not S2MM"
        );
        ensure!(self.is_running(), "DMA channel not started");
        ensure!(
            self.is_idle() || self.first_transfer,
            "DMA channel not idle"
        );
        self.write_buf_addr();
        let size = core::mem::size_of::<V>();
        let bytes = size * len;
        self.write_len(bytes as u32);

        let mut buf = Vec::with_capacity(size);

        self.wait()?;

        unsafe {
            self.udmabuf_acc.copy_to(0x00, buf.as_mut_ptr(), bytes);
            buf.set_len(bytes / size);
        }

        self.first_transfer = false;
        Ok(buf)
    }
    pub fn wait(&self) -> Result<()> {
        ensure!(self.is_running(), "DMA channel not started");
        loop {
            if self.is_error() {
                let error = unsafe { self.uio_acc.read_mem32(DMASR) };
                ensure!(error & 0x10 == 0, "DMA Internal Error (transfer length 0?)");
                ensure!(
                    error & 0x20 == 0,
                    "DMA Slave Error (cannot access memory map interface)"
                );
                ensure!(error & 0x20 == 0, "DMA Decode Error (invalid address)");
            }
            if self.is_idle() {
                break;
            }
        }
        Ok(())
    }
}

pub struct AxiDma {
    pub mm2s: Option<AxiDmaChannel>,
    pub s2mm: Option<AxiDmaChannel>,
}

impl AxiDma {
    pub fn new(hw_info: &serde_json::Value) -> Result<Self> {
        let hw_object = json_as_map!(hw_info);
        let hw_params = json_as_map!(hw_object["params"]);
        let vendor = json_as_str!(hw_object["vendor"]);
        let library = json_as_str!(hw_object["library"]);
        let name = json_as_str!(hw_object["name"]);
        ensure!(
            vendor == "xilinx.com" && library == "ip" && name == "axi_dma",
            "AxiDma::new(): This IP is not supported. ({})",
            name
        );
        let udmabuf_names = json_as_vec!(hw_object["udmabuf"]);
        let mut udmabuf_i = 0;
        let mm2s = if hw_params["C_INCLUDE_MM2S"] == 0 {
            None
        } else {
            let udmabuf_name = json_as_str!(udmabuf_names[udmabuf_i]);
            udmabuf_i += 1;
            Some(AxiDmaChannel::new(
                hw_info,
                DmaChannelMode::MM2S,
                udmabuf_name,
            )?)
        };

        let s2mm = if hw_params["C_INCLUDE_S2MM"] == 0 {
            None
        } else {
            let udmabuf_name = json_as_str!(udmabuf_names[udmabuf_i]);
            Some(AxiDmaChannel::new(
                hw_info,
                DmaChannelMode::S2MM,
                udmabuf_name,
            )?)
        };

        Ok(AxiDma { mm2s, s2mm })
    }

    pub fn start(&mut self) {
        if let Some(ch) = &mut self.mm2s {
            ch.start();
        }
        if let Some(ch) = &mut self.s2mm {
            ch.start();
        }
    }
    pub fn stop(&self) {
        if let Some(ch) = &self.mm2s {
            ch.stop();
        }
        if let Some(ch) = &self.s2mm {
            ch.stop();
        }
    }
    pub fn write<V>(&mut self, data: &[V]) -> Result<()> {
        if let Some(ch) = &mut self.mm2s {
            ch.write(data)?;
        } else {
            bail!("The MM2S channel is not supported on this IP.");
        }
        Ok(())
    }
    pub fn write_with_size<V>(&mut self, data: &[V], size: usize) -> Result<()> {
        if let Some(ch) = &mut self.mm2s {
            ch.write_with_size(data, size)?;
        } else {
            bail!("The MM2S channel is not supported on this IP.");
        }
        Ok(())
    }
    pub fn read<V>(&mut self, len: usize) -> Result<Vec<V>> {
        if let Some(ch) = &mut self.s2mm {
            ch.read(len)
        } else {
            bail!("The S2MM channel is not supported on this IP.");
        }
    }
    pub fn is_mm2s_running(&self) -> Result<bool> {
        if let Some(ch) = &self.mm2s {
            Ok(ch.is_running())
        } else {
            bail!("The MM2S channel is not supported on this IP.");
        }
    }
    pub fn is_mm2s_idle(&self) -> Result<bool> {
        if let Some(ch) = &self.mm2s {
            Ok(ch.is_idle())
        } else {
            bail!("The MM2S channel is not supported on this IP.");
        }
    }
    pub fn is_mm2s_error(&self) -> Result<bool> {
        if let Some(ch) = &self.mm2s {
            Ok(ch.is_error())
        } else {
            bail!("The MM2S channel is not supported on this IP.");
        }
    }
    pub fn is_s2mm_running(&self) -> Result<bool> {
        if let Some(ch) = &self.s2mm {
            Ok(ch.is_running())
        } else {
            bail!("The S2MM channel is not supported on this IP.");
        }
    }
    pub fn is_s2mm_idle(&self) -> Result<bool> {
        if let Some(ch) = &self.s2mm {
            Ok(ch.is_idle())
        } else {
            bail!("The S2MM channel is not supported on this IP.");
        }
    }
    pub fn is_s2mm_error(&self) -> Result<bool> {
        if let Some(ch) = &self.s2mm {
            Ok(ch.is_error())
        } else {
            bail!("The S2MM channel is not supported on this IP.");
        }
    }
}
