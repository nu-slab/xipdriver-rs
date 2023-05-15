
use anyhow::{Result, bail};

#[cfg(all(target_os = "linux", target_arch = "aarch64"))]
pub use jelly_mem_access::*;
#[cfg(all(target_os = "linux", target_arch = "aarch64"))]
pub fn new(name: &str) -> Result<UioAccessor<usize>> {
    match UioAccessor::<usize>::new_with_name(name) {
        Ok(uio_acc) => {
            Ok(uio_acc)
        },
        Err(e) => {
            bail!("{}", e)
        }
    }
}

#[cfg(not(all(target_os = "linux", target_arch = "aarch64")))]
use std::{error::Error, marker::PhantomData};
#[cfg(not(all(target_os = "linux", target_arch = "aarch64")))]
#[derive(Clone, Debug)]
pub struct UioAccessor<U> {
    uio_num: usize,
    name: String,
    phantom: PhantomData<U>,
}

#[cfg(not(all(target_os = "linux", target_arch = "aarch64")))]
impl<U> UioAccessor<U> {
    pub fn new(uio_num: usize) -> core::result::Result<Self, Box<dyn Error>> {
        Ok(UioAccessor { uio_num, name: "".to_string(), phantom: PhantomData })
    }
    pub fn new_with_name(name: &str) -> core::result::Result<Self, Box<dyn Error>> {
        Ok(UioAccessor { uio_num: 0, name: name.to_string(), phantom: PhantomData })
    }
    pub fn subclone(&self, offset: usize, size: usize) -> Self {
        UioAccessor { uio_num: self.uio_num + offset + size, name: self.name.clone(), phantom: PhantomData }
    }
    pub unsafe fn read_reg(&self, reg: usize) -> usize {
        println!("read {} {} {}", self.uio_num, self.name, reg);
        0
    }
    pub unsafe fn write_reg(&self, reg: usize, data: usize) {
        println!("write {} {} {} {}", self.uio_num, self.name, reg, data);
    }
}

#[cfg(not(all(target_os = "linux", target_arch = "aarch64")))]
pub fn new(name: &str) -> Result<UioAccessor<usize>> {
    match UioAccessor::<usize>::new_with_name(name) {
        Ok(uio_acc) => {
            Ok(uio_acc)
        },
        Err(e) => {
            bail!("{}", e)
        }
    }
}
