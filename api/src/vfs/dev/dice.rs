//! DICE模块，用于处理 DICE handover数据
use alloc::{vec, vec::Vec};
use core::any::Any;

use axerrno::{AxError, AxResult};
use axplat_aarch64_crosvm_virt::fdt::dice_reg;
use memory_addr::VirtAddr;
use rand_chacha::{
    ChaCha8Rng,
    rand_core::{RngCore, SeedableRng},
};
use spin::{Lazy, Mutex};
use starry_core::vfs::DeviceOps;
/// DICE节点信息
#[derive(Debug, Clone, Copy)]
pub struct DiceNodeInfo<'a> {
    /// 兼容性字符串（静态借用）
    pub _compatible: &'a str,
    /// 内存区域信息 (起始地址, 大小)
    pub regions: (VirtAddr, usize),
    /// 是否标记为no-map
    pub _no_map: bool,
}

const DICE_COMPATIBLE: &str = "kylin,open-dice";
const MAX_DICE_DATA_SIZE: usize = 0x1000; // 4KB

impl DiceNodeInfo<'static> {
    pub fn new() -> Self {
        DiceNodeInfo {
            _compatible: DICE_COMPATIBLE,
            regions: dice_reg().unwrap(),
            _no_map: false,
        }
    }

    fn sys_dice_get_handover(
        &self,
        handover_ptr: usize,
        handover_size: usize,
        handover_out_size: usize,
    ) -> AxResult<usize> {
        let (cdi_attest, cdi_seal, chain) = self.parse_handover_data()?;
        let handover_out_size_ptr = handover_out_size as *mut usize;
        let len = cdi_attest.len() + cdi_seal.len() + chain.len();

        unsafe {
            *handover_out_size_ptr = len;
        };

        if handover_size < len {
            return Err(AxError::InvalidInput);
        }

        // 安全写入输出缓冲区
        let handover_buffer =
            unsafe { core::slice::from_raw_parts_mut(handover_ptr as *mut u8, handover_size) };

        handover_buffer[..cdi_attest.len()].copy_from_slice(&cdi_attest);
        handover_buffer[cdi_attest.len()..cdi_attest.len() + cdi_seal.len()]
            .copy_from_slice(&cdi_seal);
        handover_buffer[cdi_attest.len() + cdi_seal.len()..len].copy_from_slice(&chain);

        warn!("dice : get handover success.");
        Ok(len)
    }

    fn parse_handover_data(&self) -> AxResult<(Vec<u8>, Vec<u8>, Vec<u8>)> {
        use dice::{dice_main_flow_chain_codehash, dice_parse_handover};

        let (addr, size) = self.regions;

        let handover_data = if size > MAX_DICE_DATA_SIZE || size == 0 {
            return Err(AxError::InvalidInput);
        } else {
            let mut buffer = Vec::new();
            buffer
                .try_reserve_exact(size)
                .map_err(|_| AxError::NoMemory)?;

            unsafe {
                buffer.set_len(size);
                // 安全拷贝内存
                core::ptr::copy_nonoverlapping(
                    addr.as_usize() as *const u8,
                    buffer.as_mut_ptr(),
                    size,
                );
            }
            buffer
        };
        let mut handover_buf = vec![0u8; size as usize];
        let hash: Vec<u8> = get_process_hash()?;
        let handover = dice_main_flow_chain_codehash(&handover_data, &hash, &mut handover_buf)
            .map_err(|_| AxError::InvalidInput)?;
        let (cdi_attest, cdi_seal, chain) =
            dice_parse_handover(&handover).map_err(|_| AxError::InvalidInput)?;

        Ok((cdi_attest.to_vec(), cdi_seal.to_vec(), chain.to_vec()))
    }
}

impl DeviceOps for DiceNodeInfo<'static> {
    fn read_at(&self, _buf: &mut [u8], _offset: u64) -> AxResult<usize> {
        unreachable!()
    }

    fn write_at(&self, _buf: &[u8], _offset: u64) -> AxResult<usize> {
        unreachable!()
    }

    fn ioctl(&self, cmd: u32, arg: usize) -> AxResult<usize> {
        if cmd == 0x90007A00 {
            let ptr = arg as *const usize;
            let handover = unsafe { core::slice::from_raw_parts_mut(ptr as *mut usize, 3) };
            return self.sys_dice_get_handover(handover[0], handover[1], handover[2]);
        }
        Err(AxError::InvalidInput)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

fn get_process_hash() -> AxResult<Vec<u8>> {
    use alloc::format;

    use axfs_ng::FS_CONTEXT;
    use axtask::current;
    use mbedtls::hash::{Md, Type};
    use starry_core::task::AsThread;

    let pid = current().as_thread().proc_data.proc.pid();
    let proc_exe_path = format!("/proc/{}/exe", pid);
    let fs = FS_CONTEXT.lock();
    let data = fs.read(proc_exe_path).unwrap();

    let mut sm3_result = vec![0u8; 32];
    let mut ctx = Md::new(Type::SM3).map_err(|_| AxError::InvalidInput)?;
    ctx.update(&data).map_err(|_| AxError::InvalidInput)?;
    let _len = ctx.finish(&mut sm3_result);

    info!("resm3_resultsult: {:x?}", sm3_result);
    Ok(sm3_result)
}

static GLOBAL_RAND: Lazy<Mutex<ChaCha8Rng>> = Lazy::new(|| {
    let seed = axhal::time::current_ticks();
    Mutex::new(ChaCha8Rng::seed_from_u64(seed))
});

#[unsafe(no_mangle)]
pub extern "C" fn get_rand(output: usize, len: usize) -> u32 {
    let buf = unsafe { core::slice::from_raw_parts_mut(output as *mut u8, len) };
    let mut rand = GLOBAL_RAND.lock();
    rand.fill_bytes(buf);
    return 0;
}
