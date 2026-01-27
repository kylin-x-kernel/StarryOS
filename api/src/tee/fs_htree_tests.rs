// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2025 KylinSoft Co., Ltd. <https://www.kylinos.cn/>
// See LICENSES for license details.
//
// This file has been created by KylinSoft on 2025.

use alloc::{boxed::Box, vec, vec::Vec};
use core::{any::Any, fmt, fmt::Debug, mem::size_of, slice};

use spin::Mutex;
use static_assertions::const_assert;
use tee_raw_sys::{TEE_ERROR_GENERIC, TEE_ERROR_SECURITY, TEE_ERROR_TIME_NOT_SET, TEE_UUID};

use super::{
    TeeResult,
    common::file_ops::FileVariant,
    fs_htree::{
        TEE_FS_HTREE_HASH_SIZE, TeeFsHtree, TeeFsHtreeImage, TeeFsHtreeNodeImage, TeeFsHtreeType,
        print_tree_hash, tee_fs_htree_close, tee_fs_htree_open, tee_fs_htree_read_block,
        tee_fs_htree_sync_to_storage, tee_fs_htree_write_block,
    },
    tee_ree_fs::TeeFsHtreeStorageOps,
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

impl Debug for test_htree_storage_inner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "test_htree_storage_inner: data.len(): {:X?}, data_len: {:X?}, data_alloced: {:X?}, \
             block.len(): {:X?}",
            self.data.len(),
            self.data_len,
            self.data_alloced,
            self.block.len()
        )
    }
}

#[derive(Debug)]
pub struct test_htree_storage {
    inner: Mutex<test_htree_storage_inner>,
}

impl Clone for test_htree_storage {
    fn clone(&self) -> Self {
        test_htree_storage {
            inner: Mutex::new(self.inner.lock().clone()),
        }
    }
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
        tee_debug!(
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
        tee_debug!(
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
            tee_debug!("!!!!!!set inner.data_len to {:X?}", end);
            inner.data_len = end;
        }

        // TODO: is this necessary?
        Ok(data.len())
    }

    fn clone_box(&self) -> Box<dyn TeeFsHtreeStorageOps> {
        Box::new(self.clone())
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

    tee_debug!("storage: {:?}", *storage);
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

    info!("------ Close and reopen the hash-tree ------");
    let storage = ht.storage.clone_box();
    tee_fs_htree_close(ht);
    tee_debug!("storage: {:?}", storage.as_ref());
    let mut ht = tee_fs_htree_open(storage, false, Some(&mut hash), Some(&TEE_UUID::default()))
        .inspect_err(|e| {
            error!("tee_fs_htree_open: error: {:X?}", e);
        })?;

    info!("------ Verify that all blocks are read as expected. ------");
    do_range(read_block, &mut ht, 0, num_blocks, salt as u8)?;

    info!("------ Rewrite a few blocks and verify that all blocks are read as expected. ------");
    do_range_backwards(
        write_block,
        &mut ht,
        w_unsync_begin,
        w_unsync_num,
        (salt + 1) as u8,
    )?;
    do_range(read_block, &mut ht, 0, w_unsync_begin, salt as u8)?;
    do_range(
        read_block,
        &mut ht,
        w_unsync_begin,
        w_unsync_num,
        (salt + 1) as u8,
    )?;
    do_range(
        read_block,
        &mut ht,
        w_unsync_begin + w_unsync_num,
        num_blocks - (w_unsync_begin + w_unsync_num),
        salt as u8,
    )?;

    info!(
        "------ Rewrite the blocks from above again with another salt and verify that they are \
         read back as expected. ------"
    );
    do_range(
        write_block,
        &mut ht,
        w_unsync_begin,
        w_unsync_num,
        (salt + 2) as u8,
    )?;
    do_range(read_block, &mut ht, 0, w_unsync_begin, salt as u8)?;
    do_range(
        read_block,
        &mut ht,
        w_unsync_begin,
        w_unsync_num,
        (salt + 2) as u8,
    )?;
    do_range(
        read_block,
        &mut ht,
        w_unsync_begin + w_unsync_num,
        num_blocks - (w_unsync_begin + w_unsync_num),
        salt as u8,
    )?;

    info!(
        "------ Skip tee_fs_htree_sync_to_storage() and call tee_fs_htree_close() directly to \
         undo the changes since last call to tee_fs_htree_sync_to_storage(). Reopen the hash-tree \
         and verify that recent changes indeed was discarded. ------"
    );
    let storage = ht.storage.clone_box();
    tee_fs_htree_close(ht);
    let mut ht = tee_fs_htree_open(storage, false, Some(&mut hash), Some(&TEE_UUID::default()))?;
    do_range(read_block, &mut ht, 0, num_blocks, salt as u8)?;

    info!(
        "------ Close, reopen and verify that all blocks are read as expected again but this time \
         based on the counter value in struct tee_fs_htree_image. ------"
    );
    let storage = ht.storage.clone_box();
    tee_fs_htree_close(ht);
    let mut ht = tee_fs_htree_open(storage, false, None, Some(&TEE_UUID::default()))?;
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

fn test_corrupt_type(
    uuid: &TEE_UUID,
    hash: &mut [u8; TEE_FS_HTREE_HASH_SIZE],
    num_blocks: usize,
    aux: &mut test_htree_storage,
    typ: TeeFsHtreeType,
    idx: usize,
) -> TeeResult {
    let mut offs: usize = 0;
    let mut size: usize = 0;
    let mut size0: usize = 0;

    (offs, size0) = test_get_offs_size(typ, idx, 0)?;

    tee_debug!(
        "test_corrupt_type: typ: {:?}, idx: {:?}, offs: {:X?}, size0: {:X?}",
        typ,
        idx,
        offs,
        size0
    );

    let mut n: usize = 0;
    let res = (|| -> TeeResult {
        let mut result: TeeResult = Ok(());

        let result = loop {
            let mut aux2 = aux.clone();
            {
                let aux_inner = aux.inner.lock();
                let mut aux2_inner = aux2.inner.lock();

                // aux2_inner.data[..aux_inner.data_len]
                //     .copy_from_slice(&aux_inner.data[..aux_inner.data_len]);

                (offs, size) = test_get_offs_size(typ, idx, 0)?;
                tee_debug!(
                    "change aux2_inner in index {:X?} with idx: {:X?}, n: {:X?}",
                    offs + n,
                    idx,
                    n
                );
                aux2_inner.data[offs + n] += 1;
                (offs, size) = test_get_offs_size(typ, idx, 1)?;
                tee_debug!(
                    "change aux2_inner in index {:X?} with idx: {:X?}, n: {:X?}",
                    offs + n,
                    idx,
                    n
                );
                aux2_inner.data[offs + n] += 1;
            }

            // Errors in head or node is detected by
            // tee_fs_htree_open() errors in block is detected when
            // actually read by do_range(read_block)
            let result = tee_fs_htree_open(Box::new(aux2), false, Some(hash), Some(uuid));
            tee_debug!("tee_fs_htree_open: result: {:?}", result);
            if result.is_ok() {
                let mut ht = result.unwrap();
                let result = do_range(read_block, &mut ht, 0, num_blocks, 1);
                // do_range(read_block,) is supposed to detect the
                // error. If TEE_ERROR_TIME_NOT_SET is returned
                // read_block() was acutally able to get some data,
                // but the data was incorrect.
                //
                // If res == TEE_SUCCESS or
                //    res == TEE_ERROR_TIME_NOT_SET
                // there's some problem with the htree
                // implementation.
                if result.is_err() && result.unwrap_err() == TEE_ERROR_TIME_NOT_SET {
                    error!("error: data silently corrupted");
                    return Err(TEE_ERROR_TIME_NOT_SET);
                }
                if result.is_ok() {
                    break Ok(());
                }

                tee_fs_htree_close(ht)?;
            }
            // We've tested the last byte, let's get out of here
            if n == size0 - 1 {
                break Err(TEE_ERROR_GENERIC);
            }

            // Increase n exponentionally after 1 to skip some testing
            if n != 0 {
                n += n;
            } else {
                n = 1;
            }

            if n >= size0 {
                n = size0 - 1;
            }
        };

        result
    })();

    if res.is_err() {
        if res.unwrap_err() == TEE_ERROR_TIME_NOT_SET {
            return Err(TEE_ERROR_TIME_NOT_SET);
        }
        return Ok(());
    } else {
        error!("error: data corruption undetected");
        return Err(TEE_ERROR_SECURITY);
    }
}

fn test_corrupt(num_blocks: usize) -> TeeResult {
    let mut aux = aux_alloc(num_blocks)?;
    let mut hash = [0u8; TEE_FS_HTREE_HASH_SIZE];
    let uuid = TEE_UUID::default();

    {
        let mut aux_inner = aux.inner.lock();
        aux_inner.data_len = 0;
        let alloced = aux_inner.data_alloced;
        aux_inner.data[..alloced].fill(0xce);
    }

    // Write the object and close it
    tee_debug!("--- Write the object and close it ---");
    let mut ht = tee_fs_htree_open(Box::new(aux), true, Some(&mut hash), Some(&uuid))?;
    do_range(write_block, &mut ht, 0, num_blocks, 1)?;
    tee_fs_htree_sync_to_storage(&mut ht, Some(&mut hash))?;

    let mut aux = ht.storage.clone_box();
    tee_fs_htree_close(ht)?;

    // Verify that the object can be read correctly
    tee_debug!("--- Verify that the object can be read correctly ---");
    let mut ht = tee_fs_htree_open(aux, false, Some(&mut hash), Some(&uuid))?;
    tee_debug!("tee_fs_htree_open: ht: {:?}", &ht);
    do_range(read_block, &mut ht, 0, num_blocks, 1)?;
    let aux = ht.storage.clone_box();
    tee_fs_htree_close(ht)?;

    // Downcast Box<dyn TeeFsHtreeStorageOps> to Box<test_htree_storage>
    // We know in test context, storage is always test_htree_storage
    // First convert to Box<dyn Any>, then downcast
    tee_debug!("--- test_corrupt with Head ---");
    let aux_any: Box<dyn core::any::Any> = aux;
    let aux_box: Box<test_htree_storage> = aux_any.downcast().map_err(|_| TEE_ERROR_GENERIC)?;
    let mut aux = *aux_box;
    test_corrupt_type(
        &uuid,
        &mut hash,
        num_blocks,
        &mut aux,
        TeeFsHtreeType::Head,
        0,
    )?;

    tee_debug!("--- test_corrupt with Node ---");
    for n in 0..num_blocks {
        tee_debug!("--- test in loop with num_blocks : {} ---", n);
        test_corrupt_type(
            &uuid,
            &mut hash,
            num_blocks,
            &mut aux,
            TeeFsHtreeType::Node,
            n,
        )?;
    }

    tee_debug!("--- test_corrupt with Block ---");
    for n in 0..num_blocks {
        tee_debug!("--- test in loop with num_blocks : {} ---", n);
        test_corrupt_type(
            &uuid,
            &mut hash,
            num_blocks,
            &mut aux,
            TeeFsHtreeType::Block,
            n,
        )?;
    }

    Ok(())
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
            let result = test_write_read(10);
            assert!(result.is_ok());

            let result = test_corrupt(5);
            assert!(result.is_ok());
        }
    }

    tests_name! {
        TEST_FS_HTREE_TESTS;
        //------------------------
        core_fs_htree_tests,
    }
}
