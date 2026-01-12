// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2025 KylinSoft Co., Ltd. <https://www.kylinos.cn/>
// See LICENSES for license details.
//
// This file has been created by KylinSoft on 2025.

use alloc::{boxed::Box, vec, vec::Vec};
use core::{mem::size_of, slice};

use spin::Mutex;
use static_assertions::const_assert;
use tee_raw_sys::{TEE_ERROR_GENERIC, TEE_ERROR_TIME_NOT_SET, TEE_UUID};

use super::{
    TeeResult,
    common::file_ops::FileVariant,
    fs_htree::{
        TEE_FS_HTREE_HASH_SIZE, TeeFsHtree, TeeFsHtreeImage, TeeFsHtreeNodeImage, TeeFsHtreeType,
        print_tree_hash, tee_fs_htree_open, tee_fs_htree_read_block, tee_fs_htree_sync_to_storage,
        tee_fs_htree_write_block,
    },
    tee_fs_key_manager::tee_fs_init_key_manager,
    tee_ree_fs::TeeFsHtreeStorageOps,
    tee_session::tee_session_set_current_uuid,
    utils::shift_u32,
};
// The smallest blocks size that can hold two struct
// tee_fs_htree_node_image or two struct tee_fs_htree_image.
const TEST_BLOCK_SIZE: usize = 144;

pub fn test_get_offs_size(typ: TeeFsHtreeType, idx: usize, vers: u8) -> TeeResult<(usize, usize)> {
    let node_size = size_of::<TeeFsHtreeNodeImage>();
    let block_nodes = TEST_BLOCK_SIZE / (node_size * 2);

    const_assert!(TEST_BLOCK_SIZE > size_of::<TeeFsHtreeNodeImage>() * 2);
    const_assert!(TEST_BLOCK_SIZE > size_of::<TeeFsHtreeImage>() * 2);

    debug_assert!(vers == 0 || vers == 1);

    // File layout
    //
    // phys block 0:
    // tee_fs_htree_image vers 0 @ offs = 0
    // tee_fs_htree_image vers 1 @ offs = sizeof(tee_fs_htree_image)
    //
    // phys block 1:
    // tee_fs_htree_node_image 0  vers 0 @ offs = 0
    // tee_fs_htree_node_image 0  vers 1 @ offs = node_size
    //
    // phys block 2:
    // data block 0 vers 0
    //
    // phys block 3:
    // tee_fs_htree_node_image 1  vers 0 @ offs = 0
    // tee_fs_htree_node_image 1  vers 1 @ offs = node_size
    //
    // phys block 4:
    // data block 0 vers 1
    //
    // ...

    match typ {
        TeeFsHtreeType::Head => {
            let offs = size_of::<TeeFsHtreeImage>() * vers as usize;
            let size = size_of::<TeeFsHtreeImage>();
            Ok((offs, size))
        }
        TeeFsHtreeType::Node => {
            let pbn = 1 + ((idx / block_nodes) * block_nodes * 2);
            let offs = pbn * TEST_BLOCK_SIZE
                + 2 * node_size * (idx % block_nodes)
                + node_size * vers as usize;
            Ok((offs, node_size))
        }
        TeeFsHtreeType::Block => {
            let bidx = 2 * idx + vers as usize;
            let pbn = 2 + bidx + bidx / (block_nodes * 2 - 1);
            Ok((pbn * TEST_BLOCK_SIZE, TEST_BLOCK_SIZE))
        }
        _ => Err(TEE_ERROR_GENERIC),
    }
}

#[derive(Clone)]
pub struct test_htree_storage_inner {
    pub data: Vec<u8>,
    pub data_len: usize,
    pub data_alloced: usize,
    pub block: Vec<u8>,
}

pub struct test_htree_storage {
    inner: Mutex<test_htree_storage_inner>,
}

impl TeeFsHtreeStorageOps for test_htree_storage {
    fn block_size(&self) -> usize {
        TEST_BLOCK_SIZE
    }

    fn rpc_read_init(&self) -> TeeResult {
        Ok(())
    }

    fn rpc_read_final(
        &self,
        typ: TeeFsHtreeType,
        idx: usize,
        vers: u8,
        data: &mut [u8],
    ) -> TeeResult<usize> {
        warn!(
            "rpc_read_final: typ: {:?}, idx: {:?}, vers: {:?}, data_len: {:X?}",
            typ,
            idx,
            vers,
            data.len()
        );
        let (offs, size) = test_get_offs_size(typ, idx, vers)?;

        let mut inner = self.inner.lock();
        let bytes = if offs + size <= inner.data_len {
            size
        } else if offs <= inner.data_len {
            inner.data_len - offs
        } else {
            0
        };

        tee_debug!(
            "rpc_read_final: offs: {:X?}, size: {:X?}, bytes: {:X?}",
            offs,
            size,
            bytes
        );
        let src_data = inner.data[offs..offs + bytes].to_vec();
        inner.block[..bytes].copy_from_slice(&src_data);

        // copy to data
        data.copy_from_slice(&inner.block[..data.len()]);

        tee_debug!("rpc_read_final: bytes: {:X?}", bytes);
        Ok(bytes)
    }

    fn rpc_write_init(&self) -> TeeResult {
        Ok(())
    }

    fn rpc_write_final(
        &self,
        typ: TeeFsHtreeType,
        idx: usize,
        vers: u8,
        data: &[u8],
    ) -> TeeResult<usize> {
        warn!(
            "rpc_write_final: typ: {:?}, idx: {:?}, vers: {:?}, data_len: {:X?}",
            typ,
            idx,
            vers,
            data.len()
        );

        let (offs, sz) = test_get_offs_size(typ, idx, vers)?;
        let end = offs + sz;

        let mut inner = self.inner.lock();
        // copy data to inner.block
        inner.block[..data.len()].copy_from_slice(data);

        if end > inner.data_alloced {
            error!("out of bounds");
            return Err(TEE_ERROR_GENERIC);
        }

        tee_debug!(
            "rpc_write_final: offs: {:X?}, size: {:X?}, bytes: {:X?}",
            offs,
            sz,
            data.len()
        );

        let src_block = inner.block.to_vec();
        inner.data[offs..end].copy_from_slice(&src_block[..sz]);

        if end > inner.data_len {
            inner.data_len = end;
        }

        // TODO: is this necessary?
        Ok(data.len())
    }
}

fn val_from_bn_n_salt(bn: usize, n: usize, salt: u8) -> u32 {
    debug_assert!(bn < u16::MAX as usize);
    debug_assert!(n < u8::MAX as usize);
    shift_u32(n as u32, 16) | shift_u32(bn as u32, 8) | salt as u32
}

fn write_block(ht: &mut TeeFsHtree, bn: usize, salt: u8) -> TeeResult {
    let mut b = [0u32; TEST_BLOCK_SIZE / size_of::<u32>()];
    let mut n = 0;

    for n in 0..b.len() {
        b[n] = val_from_bn_n_salt(bn, n, salt);
    }

    let bytes: &[u8] =
        unsafe { slice::from_raw_parts(b.as_ptr() as *const u8, core::mem::size_of_val(&b)) };

    debug_assert!(bytes.len() == TEST_BLOCK_SIZE);
    // let storage = ht.storage.as_ref();
    tee_fs_htree_write_block(ht, bn, bytes)
}

fn read_block(ht: &mut TeeFsHtree, bn: usize, salt: u8) -> TeeResult {
    let mut b = [0u32; TEST_BLOCK_SIZE / size_of::<u32>()];
    let mut n = 0;

    let mut bytes: &mut [u8] =
        unsafe { slice::from_raw_parts_mut(b.as_ptr() as *mut u8, core::mem::size_of_val(&b)) };

    // let storage = ht.storage.as_ref();
    tee_fs_htree_read_block(ht, bn, bytes)?;

    for n in 0..b.len() {
        if b[n] != val_from_bn_n_salt(bn, n, salt) {
            error!(
                "Unexpected b[{}]: {:X} (expected {:X})",
                n,
                b[n],
                val_from_bn_n_salt(bn, n, salt)
            );
            return Err(TEE_ERROR_TIME_NOT_SET);
        }
    }

    Ok(())
}

fn do_range(
    f: fn(&mut TeeFsHtree, usize, u8) -> TeeResult,
    ht: &mut TeeFsHtree,
    begin: usize,
    num_blocks: usize,
    salt: u8,
) -> TeeResult {
    for n in 0..num_blocks {
        f(ht, n + begin, salt)?;
    }
    Ok(())
}

fn do_range_backwards(
    f: fn(&mut TeeFsHtree, usize, u8) -> TeeResult,
    ht: &mut TeeFsHtree,
    begin: usize,
    num_blocks: usize,
    salt: u8,
) -> TeeResult {
    for n in 0..num_blocks {
        f(ht, num_blocks - 1 - n + begin, salt)?;
    }
    Ok(())
}

fn htree_test_rewrite(
    aux: &mut test_htree_storage,
    num_blocks: usize,
    w_unsync_begin: usize,
    w_unsync_num: usize,
) -> TeeResult {
    let mut salt: usize = 23;
    let mut hash = [0u8; TEE_FS_HTREE_HASH_SIZE];

    debug_assert!((w_unsync_begin + w_unsync_num) <= num_blocks);
    let mut aux_inner = aux.inner.lock();
    aux_inner.data_len = 0;
    let alloced = aux_inner.data_alloced;
    aux_inner.data[..alloced].fill(0xce);

    let storage = Box::new(test_htree_storage {
        inner: Mutex::new(aux_inner.clone()),
    });
    let mut ht = tee_fs_htree_open(storage, true, Some(&mut hash), Some(&TEE_UUID::default()))?;

    // Intialize all blocks and verify that they read back as
    // expected.
    info!("------ Initialize all blocks and verify that they read back as expected. ------");
    do_range(write_block, &mut ht, 0, num_blocks, salt as u8)?;
    do_range(read_block, &mut ht, 0, num_blocks, salt as u8)?;

    // Write all blocks again, but starting from the end using a new
    // salt, then verify that that read back as expected.
    info!(
        "------ Write all blocks again, but starting from the end using a new salt, then verify \
         that that read back as expected. ------"
    );
    salt += 1;
    do_range_backwards(write_block, &mut ht, 0, num_blocks, salt as u8)?;
    do_range(read_block, &mut ht, 0, num_blocks, salt as u8)?;

    // Use a new salt to write all blocks once more and verify that
    // they read back as expected.
    salt += 1;
    info!(
        "------ Use a new salt to write all blocks once more and verify that they read back as \
         expected. ------"
    );
    do_range(write_block, &mut ht, 0, num_blocks, salt as u8)?;
    do_range(read_block, &mut ht, 0, num_blocks, salt as u8)?;

    print_tree_hash(&ht)?;
    // Sync the changes of the nodes to memory, verify that all
    // blocks are read back as expected.
    info!(
        "------ Sync the changes of the nodes to memory, verify that all blocks are read back as \
         expected. ------"
    );
    tee_fs_htree_sync_to_storage(&mut ht, Some(&mut hash))?;

    do_range(read_block, &mut ht, 0, num_blocks, salt as u8)?;

    Ok(())
}

fn aux_alloc(num_blocks: usize) -> TeeResult<test_htree_storage> {
    let (offs, sz) = test_get_offs_size(TeeFsHtreeType::Block, num_blocks, 1)?;
    let aux = test_htree_storage {
        inner: Mutex::new(test_htree_storage_inner {
            data: vec![0; offs + sz],
            data_len: 0,
            data_alloced: offs + sz,
            block: vec![0; TEST_BLOCK_SIZE],
        }),
    };
    Ok(aux)
}

fn test_write_read(num_blocks: usize) -> TeeResult {
    let mut aux = aux_alloc(num_blocks)?;

    for n in (0..num_blocks).step_by(3) {
        for m in (0..n).step_by(3) {
            for o in (0..(n - m)).step_by(3) {
                info!("test_write_read: n: {}, m: {}, o: {}", n, m, o);
                htree_test_rewrite(&mut aux, n, m, o)?;
            }
        }
    }

    Ok(())
}

#[cfg(feature = "tee_test")]
pub mod tests_fs_htree_tests {
    //-------- test framework import --------
    //-------- local tests import --------
    use super::*;
    use crate::{
        assert, assert_eq, assert_ne,
        tee::{TestDescriptor, TestResult},
        test_fn, tests, tests_name,
    };

    test_fn! {
        using TestResult;

        fn core_fs_htree_tests() {
            tee_session_set_current_uuid(&TEE_UUID {
                timeLow: 0,
                timeMid: 0,
                timeHiAndVersion: 0,
                clockSeqAndNode: [0; 8],
            });

            let res = tee_fs_init_key_manager();
            assert!(res.is_ok());

            let result = test_write_read(10);
            assert!(result.is_ok());
        }
    }

    tests_name! {
        TEST_FS_HTREE_TESTS;
        //------------------------
        core_fs_htree_tests,
    }
}
