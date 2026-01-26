// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2025 KylinSoft Co., Ltd. <https://www.kylinos.cn/>
// Copyright (C) 2025 Yuekai Jia <equation618@gmail.com>
// Copyright (C) 2025 ChengXiang Qi <kuangjux@outlook.com>
// See LICENSE for license details.
//
// This file has been modified by KylinSoft on 2025.

use core::{marker::PhantomData, ptr::NonNull};

use axalloc::{UsageKind, global_allocator};
use axdriver_base::{BaseDriverOps, DevResult, DeviceType};
use axdriver_virtio::{BufferDirection, PhysAddr, VirtIoHal};
use axhal::mem::{phys_to_virt, virt_to_phys};
#[cfg(feature = "crosvm")]
use axhal::psci::{share_dma_buffer, unshare_dma_buffer};
use cfg_if::cfg_if;

use crate::{AxDeviceEnum, drivers::DriverProbe};

cfg_if! {
    if #[cfg(bus = "pci")] {
        use axdriver_pci::{PciRoot, DeviceFunction, DeviceFunctionInfo};
        type VirtIoTransport = axdriver_virtio::PciTransport;
    } else if #[cfg(bus =  "mmio")] {
        type VirtIoTransport = axdriver_virtio::MmioTransport;
    }
}

/// A trait for VirtIO device meta information.
pub trait VirtIoDevMeta {
    const DEVICE_TYPE: DeviceType;

    type Device: BaseDriverOps;
    type Driver = VirtIoDriver<Self>;

    fn try_new(transport: VirtIoTransport, irq: Option<usize>) -> DevResult<AxDeviceEnum>;
}

cfg_if! {
    if #[cfg(net_dev = "virtio-net")] {
        pub struct VirtIoNet;

        impl VirtIoDevMeta for VirtIoNet {
            const DEVICE_TYPE: DeviceType = DeviceType::Net;
            type Device = axdriver_virtio::VirtIoNetDev<VirtIoHalImpl, VirtIoTransport, 64>;

            fn try_new(transport: VirtIoTransport, irq: Option<usize>) -> DevResult<AxDeviceEnum> {
                Ok(AxDeviceEnum::from_net(Self::Device::try_new(transport, irq)?))
            }
        }
    }
}

cfg_if! {
    if #[cfg(block_dev = "virtio-blk")] {
        pub struct VirtIoBlk;

        impl VirtIoDevMeta for VirtIoBlk {
            const DEVICE_TYPE: DeviceType = DeviceType::Block;
            type Device = axdriver_virtio::VirtIoBlkDev<VirtIoHalImpl, VirtIoTransport>;

            fn try_new(transport: VirtIoTransport, _irq: Option<usize>) -> DevResult<AxDeviceEnum> {
                Ok(AxDeviceEnum::from_block(Self::Device::try_new(transport)?))
            }
        }
    }
}

cfg_if! {
    if #[cfg(display_dev = "virtio-gpu")] {
        pub struct VirtIoGpu;

        impl VirtIoDevMeta for VirtIoGpu {
            const DEVICE_TYPE: DeviceType = DeviceType::Display;
            type Device = axdriver_virtio::VirtIoGpuDev<VirtIoHalImpl, VirtIoTransport>;

            fn try_new(transport: VirtIoTransport, _irq: Option<usize>) -> DevResult<AxDeviceEnum> {
                Ok(AxDeviceEnum::from_display(Self::Device::try_new(transport)?))
            }
        }
    }
}

cfg_if! {
    if #[cfg(input_dev = "virtio-input")] {
        pub struct VirtIoInput;

        impl VirtIoDevMeta for VirtIoInput {
            const DEVICE_TYPE: DeviceType = DeviceType::Input;
            type Device = axdriver_virtio::VirtIoInputDev<VirtIoHalImpl, VirtIoTransport>;

            fn try_new(transport: VirtIoTransport, _irq: Option<usize>) -> DevResult<AxDeviceEnum> {
                Ok(AxDeviceEnum::from_input(Self::Device::try_new(transport)?))
            }
        }
    }
}

cfg_if! {
    if #[cfg(vsock_dev = "virtio-socket")] {
        pub struct VirtIoSocket;

        impl VirtIoDevMeta for VirtIoSocket {
            const DEVICE_TYPE: DeviceType = DeviceType::Vsock;
            type Device = axdriver_virtio::VirtIoSocketDev<VirtIoHalImpl, VirtIoTransport>;

            fn try_new(transport: VirtIoTransport, _irq:  Option<usize>) -> DevResult<AxDeviceEnum> {
                Ok(AxDeviceEnum::from_vsock(Self::Device::try_new(transport)?))
            }
        }
    }
}

/// A common driver for all VirtIO devices that implements [`DriverProbe`].
pub struct VirtIoDriver<D: VirtIoDevMeta + ?Sized>(PhantomData<D>);

impl<D: VirtIoDevMeta> DriverProbe for VirtIoDriver<D> {
    #[cfg(bus = "mmio")]
    fn probe_mmio(mmio_base: usize, mmio_size: usize) -> Option<AxDeviceEnum> {
        let base_vaddr = phys_to_virt(mmio_base.into());
        if let Some((ty, transport)) =
            axdriver_virtio::probe_mmio_device(base_vaddr.as_mut_ptr(), mmio_size)
            && ty == D::DEVICE_TYPE
        {
            match D::try_new(transport, None) {
                Ok(dev) => return Some(dev),
                Err(e) => {
                    warn!(
                        "failed to initialize MMIO device at [PA:{:#x}, PA:{:#x}): {:?}",
                        mmio_base,
                        mmio_base + mmio_size,
                        e
                    );
                    return None;
                }
            }
        }
        None
    }

    #[cfg(bus = "pci")]
    fn probe_pci(
        root: &mut PciRoot,
        bdf: DeviceFunction,
        dev_info: &DeviceFunctionInfo,
    ) -> Option<AxDeviceEnum> {
        if dev_info.vendor_id != 0x1af4 {
            return None;
        }
        match (D::DEVICE_TYPE, dev_info.device_id) {
            (DeviceType::Net, 0x1000) | (DeviceType::Net, 0x1041) => {}
            (DeviceType::Block, 0x1001) | (DeviceType::Block, 0x1042) => {}
            (DeviceType::Input, 0x1052) => {}
            (DeviceType::Display, 0x1050) => {}
            (DeviceType::Vsock, 0x1053) => {}
            _ => return None,
        }

        if let Some((ty, transport, irq)) =
            axdriver_virtio::probe_pci_device::<VirtIoHalImpl>(root, bdf, dev_info)
            && ty == D::DEVICE_TYPE
        {
            match D::try_new(transport, Some(irq)) {
                Ok(dev) => return Some(dev),
                Err(e) => {
                    warn!("failed to initialize PCI device at {bdf}({dev_info}): {e:?}");
                    return None;
                }
            }
        }
        None
    }
}

pub struct VirtIoHalImpl;

cfg_if! {
    if #[cfg(feature = "crosvm")] {
        use hashbrown::HashMap;
        use axsync::Mutex;
        use spin::Lazy;
        const PAGE_SIZE: usize = 0x1000; // define page size as 4KB
        const VIRTIO_QUEUE_SIZE: usize = 32;

        struct VirtIoFramePool
        {
            pool_paddr: PhysAddr,
            bitmap: [bool; VIRTIO_QUEUE_SIZE],
            v2p_map: HashMap<usize, usize>,
        }

        static VIRTIO_FRAME_POOL: Lazy<Mutex<VirtIoFramePool>> = Lazy::new(|| {
            let vaddr = global_allocator().alloc_pages(VIRTIO_QUEUE_SIZE,0x1000,UsageKind::Dma).expect("virtio frame pool alloc failed");
            let paddr = virt_to_phys(vaddr.into());
            share_dma_buffer(paddr.as_usize(), VIRTIO_QUEUE_SIZE * PAGE_SIZE);
            let pool = VirtIoFramePool {
                pool_paddr: paddr.into(),
                bitmap: [false; VIRTIO_QUEUE_SIZE],
                v2p_map: HashMap::new(),
            };
            Mutex::new(pool)
        });

        impl VirtIoFramePool {
            fn alloc_page_from_pool(&mut self, vaddr: usize) -> PhysAddr {
                let frame_index = {
                    let mut fram_index = usize::MAX;
                    for i in 0..VIRTIO_QUEUE_SIZE {
                        if !self.bitmap[i] {
                            fram_index = i;
                            self.bitmap[i] = true;
                            break;
                        }
                    }
                    assert!(fram_index != usize::MAX);
                    fram_index
                };
                self.v2p_map.insert(vaddr, frame_index);
                let paddr = self.pool_paddr + (PAGE_SIZE * frame_index);
                paddr
            }

            fn free_page_to_pool(&mut self, vaddr: usize) {
                let frame_index = self.v2p_map.remove(&vaddr).unwrap();
                assert!(self.bitmap[frame_index]);
                self.bitmap[frame_index] = false;
            }
        }
    }
}

unsafe impl VirtIoHal for VirtIoHalImpl {
    fn dma_alloc(pages: usize, _direction: BufferDirection) -> (PhysAddr, NonNull<u8>) {
        let vaddr = if let Ok(vaddr) = global_allocator().alloc_pages(pages, 0x1000, UsageKind::Dma)
        {
            vaddr
        } else {
            return (0, NonNull::dangling());
        };
        let paddr = virt_to_phys(vaddr.into());
        let ptr = NonNull::new(vaddr as _).unwrap();

        #[cfg(feature = "crosvm")]
        {
            share_dma_buffer(paddr.as_usize(), pages * 0x1000);
        }
        (paddr.as_usize(), ptr)
    }

    #[allow(unused_variables)]
    unsafe fn dma_dealloc(paddr: PhysAddr, vaddr: NonNull<u8>, pages: usize) -> i32 {
        global_allocator().dealloc_pages(vaddr.as_ptr() as usize, pages, UsageKind::Dma);
        #[cfg(feature = "crosvm")]
        {
            unshare_dma_buffer(paddr as usize, pages * 0x1000);
        }
        0
    }

    #[inline]
    unsafe fn mmio_phys_to_virt(paddr: PhysAddr, _size: usize) -> NonNull<u8> {
        NonNull::new(phys_to_virt(paddr.into()).as_mut_ptr()).unwrap()
    }

    #[allow(unused_variables)]
    #[inline]
    unsafe fn share(buffer: NonNull<[u8]>, direction: BufferDirection) -> PhysAddr {
        #[cfg(feature = "crosvm")]
        {
            let vaddr = buffer.as_ptr() as *mut u8 as usize;
            let len = buffer.len();
            assert!(len <= 0x1000, "only support share buffer size <= 4KB");
            let paddr = {
                let mut pool = VIRTIO_FRAME_POOL.lock();
                pool.alloc_page_from_pool(vaddr)
            };

            if direction != BufferDirection::DeviceToDriver {
                let data = unsafe {
                    let data = phys_to_virt(paddr.into()).as_usize() as *mut u8;
                    core::slice::from_raw_parts_mut(data, len)
                };
                data.clone_from_slice(unsafe { &buffer.as_ref() });
            }
            paddr
        }

        #[cfg(not(feature = "crosvm"))]
        {
            let vaddr = buffer.as_ptr() as *mut u8 as usize;
            virt_to_phys(vaddr.into()).into()
        }
    }

    #[inline]
    #[allow(unused_variables)]
    unsafe fn unshare(paddr: PhysAddr, buffer: NonNull<[u8]>, direction: BufferDirection) {
        #[cfg(feature = "crosvm")]
        {
            let mut buffer = buffer;
            let vaddr = buffer.as_ptr() as *mut u8 as usize;

            if direction != BufferDirection::DriverToDevice {
                let data = unsafe {
                    let data = phys_to_virt(paddr.into()).as_usize() as *mut u8;
                    core::slice::from_raw_parts(data, buffer.len())
                };
                unsafe { buffer.as_mut().clone_from_slice(&data) };
            }

            let mut pool = VIRTIO_FRAME_POOL.lock();
            pool.free_page_to_pool(vaddr);
        }
    }
}
