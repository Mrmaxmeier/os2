#![allow(dead_code)]
use crate::memory::allocate_and_map_contiguous_phys_region;
use core::ops::{Deref, DerefMut};
use core::{mem, ptr};
use x86_64::PhysAddr;

pub struct Dma<T> {
    phys: PhysAddr,
    virt: *mut T,
}

impl<T> Dma<T> {
    pub fn new(value: T) -> Dma<T> {
        let (phys, virt) = allocate_and_map_contiguous_phys_region(mem::size_of::<T>() as u64);
        unsafe {
            ptr::write(virt.start() as *mut T, value);
        }
        Dma {
            phys,
            virt: virt.start() as *mut T,
        }
    }

    pub fn zeroed() -> Dma<T> {
        let (phys, virt) = allocate_and_map_contiguous_phys_region(mem::size_of::<T>() as u64);
        unsafe {
            ptr::write_bytes(virt.start(), 0, mem::size_of::<T>());
        }
        Dma {
            phys,
            virt: virt.start() as *mut T,
        }
    }

    pub fn physical(&self) -> u64 {
        self.phys.as_u64()
    }
}

impl<T> Deref for Dma<T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { &*self.virt }
    }
}

impl<T> DerefMut for Dma<T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.virt }
    }
}

impl<T> Drop for Dma<T> {
    fn drop(&mut self) {
        unsafe {
            drop(ptr::read(self.virt));
        }
        // TODO:
        // let _ = unsafe { ::physunmap(self.virt as usize) };
    }
}
