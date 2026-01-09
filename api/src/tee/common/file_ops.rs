// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2025 KylinSoft Co., Ltd. <https://www.kylinos.cn/>
// See LICENSES for license details.
//
// This file has been created by KylinSoft on 2025.
use alloc::{sync::Arc, vec::Vec};
use core::ffi::c_int;

use axerrno::{AxError, AxResult};
use axfs::{FS_CONTEXT, File, FileBackend, FileFlags, OpenOptions, OpenResult};
use axfs_ng_vfs::VfsError;
use axio::{Seek, SeekFrom};
use axtask::current;
use linux_raw_sys::general::*;
use scope_local::scope_local;
use slab::Slab;
use spin::RwLock;
use starry_core::task::AsThread;
use tee_raw_sys::TEE_ERROR_GENERIC;

use crate::{
    file::{SealedBuf, SealedBufMut, resolve_at, with_fs},
    tee::TeeResult,
};

pub const FS_MODE_644: u32 = (S_IRUSR | S_IWUSR | S_IRGRP | S_IROTH);
pub const FS_OFLAG_DEFAULT: u32 = O_CREAT | O_RDWR;
pub const FS_OFLAG_RW: u32 = O_RDWR;
pub const FS_OFLAG_RW_TRUNC: u32 = O_RDWR | O_TRUNC;

scope_local::scope_local! {
    /// The open objects for TA.
    pub static TEE_FD_TABLE: Arc<RwLock<Slab<Arc<File>>>> = Arc::default();
}

/// Convert open flags to [`OpenOptions`].
fn flags_to_options(flags: c_int, mode: __kernel_mode_t, (uid, gid): (u32, u32)) -> OpenOptions {
    let flags = flags as u32;
    let mut options = OpenOptions::new();
    options.mode(mode).user(uid, gid);
    match flags & 0b11 {
        O_RDONLY => options.read(true),
        O_WRONLY => options.write(true),
        _ => options.read(true).write(true),
    };
    if flags & O_APPEND != 0 {
        options.append(true);
    }
    if flags & O_TRUNC != 0 {
        options.truncate(true);
    }
    if flags & O_CREAT != 0 {
        options.create(true);
    }
    if flags & O_PATH != 0 {
        options.path(true);
    }
    if flags & O_EXCL != 0 {
        options.create_new(true);
    }
    if flags & O_DIRECTORY != 0 {
        options.directory(true);
    }
    if flags & O_NOFOLLOW != 0 {
        options.no_follow(true);
    }
    if flags & O_DIRECT != 0 {
        options.direct(true);
    }
    options
}

pub trait TeeFileLike {
    /// read data from file to buffer
    ///
    /// # Arguments
    /// * `buf` - buffer to store read data
    ///
    /// # Returns
    /// * `Ok(usize)` - number of bytes read
    /// * `Err(TEE_ERROR_GENERIC)` - error
    fn read(&mut self, buf: &mut [u8]) -> TeeResult<usize>;

    /// read data from file to buffer at offset
    ///
    /// # Arguments
    /// * `buf` - buffer to store read data
    /// * `offset` - offset from the beginning of the file
    ///
    /// # Returns
    /// * `Ok(usize)` - number of bytes read
    /// * `Err(TEE_ERROR_GENERIC)` - error
    fn pread(&mut self, buf: &mut [u8], offset: usize) -> TeeResult<usize>;

    /// write data to file
    ///
    /// # Arguments
    /// * `buf` - data to write
    ///
    /// # Returns
    /// * `Ok(usize)` - number of bytes written
    /// * `Err(TEE_ERROR_GENERIC)` - error
    fn write(&mut self, buf: &[u8]) -> TeeResult<usize>;

    /// write data to file at offset
    ///
    /// # Arguments
    /// * `buf` - data to write
    /// * `offset` - offset from the beginning of the file
    ///
    /// # Returns
    /// * `Ok(usize)` - number of bytes written
    /// * `Err(TEE_ERROR_GENERIC)` - error
    fn pwrite(&self, buf: &[u8], offset: usize) -> TeeResult<usize>;

    /// move file read write pointer to offset
    ///
    /// # Arguments
    /// * `pos` - `SeekFrom` enum, specify the starting position and offset of the search
    ///
    /// # Returns
    /// * `Ok(u64)` - new pointer position (number of bytes from the beginning of the file)
    /// * `Err(TEE_ERROR_GENERIC)` - error
    fn seek(&mut self, pos: SeekFrom) -> TeeResult<u64>;

    /// truncate file to length
    ///
    /// # Arguments
    /// * `len` - new file length (number of bytes)
    ///
    /// # Returns
    /// * `Ok(())` - success
    /// * `Err(TEE_ERROR_GENERIC)` - error
    fn ftruncate(&mut self, len: usize) -> TeeResult<()>;

    /// close file
    ///
    /// # Returns
    /// * `Ok(())` - success
    /// * `Err(TEE_ERROR_GENERIC)` - error
    fn close(&mut self) -> TeeResult<()>;
}
#[derive(Debug, Clone, Copy)]
pub struct FileVariant {
    pub fd: isize,
}

impl Default for FileVariant {
    fn default() -> Self {
        Self { fd: -1 }
    }
}

fn add_to_fd(result: OpenResult, flags: u32) -> AxResult<isize> {
    let f = match result {
        OpenResult::File(file) => file,
        _ => {
            info!("add_to_fd = error");
            return Err(AxError::InvalidInput);
        }
    };

    let fd = TEE_FD_TABLE.write().insert(Arc::new(f));
    Ok(fd as isize)
}

fn with_file<F, R>(file: &FileVariant, f: F) -> TeeResult<R>
where
    F: FnOnce(&Arc<File>) -> TeeResult<R>,
{
    let file_arc = TEE_FD_TABLE
        .read()
        .get(file.fd as usize)
        .ok_or_else(|| {
            error!("invalid fd {}", file.fd);
            TEE_ERROR_GENERIC
        })?
        .clone();
    f(&file_arc)
}

impl FileVariant {
    pub fn open(path: &str, flags: u32, mode: u32) -> Result<Self, VfsError> {
        tee_debug!(
            "FileVariant::open: path: {}, flags: {}, mode: {}",
            path,
            flags,
            mode
        );
        let mode = mode & !current().as_thread().proc_data.umask();

        let options = flags_to_options(flags as c_int, mode as __kernel_mode_t, (0, 0));
        let fd = with_fs(AT_FDCWD, |fs| options.open(fs, path))
            .and_then(|it| add_to_fd(it, flags as _))?;

        tee_debug!("FileVariant::open = fd: {}", fd);
        Ok(Self { fd })
    }

    /// get raw file descriptor
    pub fn as_raw_fd(&self) -> isize {
        self.fd
    }

    /// remove file
    ///
    /// # Arguments
    /// * `path` - the path of the file to remove
    /// # Returns
    /// * `TeeResult` - the result of the operation
    pub fn remove(path: &str) -> TeeResult {
        tee_debug!("FileVariant::remove file with path: {}", path);
        with_fs(AT_FDCWD, |fs| fs.remove_file(path))
            .inspect_err(|e| error!("remove file failed: {:?}", e))
            .map_err(|_| TEE_ERROR_GENERIC)?;

        Ok(())
    }

    /// check if file exists
    ///
    /// # Arguments
    /// * `path` - the path of the file to check
    /// # Returns
    /// * `bool` - true if file exists, false otherwise
    pub fn exists(path: &str) -> bool {
        let loc = resolve_at(AT_FDCWD, Some(path), AT_EMPTY_PATH);
        match loc {
            Ok(loc) => loc.stat().is_ok(),
            Err(_) => false,
        }
    }
}

impl TeeFileLike for FileVariant {
    fn read(&mut self, buf: &mut [u8]) -> TeeResult<usize> {
        with_file(self, |file| {
            file.read(&mut SealedBufMut::from(buf))
                .inspect_err(|e| error!("read from file failed: {:?}", e))
                .map_err(|_| TEE_ERROR_GENERIC)
        })
    }

    fn pread(&mut self, buf: &mut [u8], offset: usize) -> TeeResult<usize> {
        tee_debug!(
            "FileVariant::pread = fd: {}, offset: 0x{:X?}, buf_len: 0x{:X?}",
            self.fd,
            offset,
            buf.len(),
        );
        with_file(self, |file| {
            file.read_at(&mut SealedBufMut::from(buf), offset as _)
                .inspect_err(|e| error!("read_at from file failed: {:?}", e))
                .map_err(|_| TEE_ERROR_GENERIC)
        })
    }

    fn write(&mut self, buf: &[u8]) -> TeeResult<usize> {
        tee_debug!(
            "FileVariant::write = fd: {}, buf: {:x?}, len: {}",
            self.fd,
            buf,
            buf.len()
        );
        with_file(self, |file| {
            file.write(&mut SealedBuf::from(buf))
                .inspect_err(|e| error!("write to file failed: {:?}", e))
                .map_err(|_| TEE_ERROR_GENERIC)
        })
    }

    fn pwrite(&self, buf: &[u8], offset: usize) -> TeeResult<usize> {
        with_file(self, |file| {
            file.write_at(&mut SealedBuf::from(buf), offset as _)
                .inspect_err(|e| error!("write_at to file failed: {:?}", e))
                .map_err(|_| TEE_ERROR_GENERIC)
        })
    }

    fn seek(&mut self, pos: SeekFrom) -> TeeResult<u64> {
        with_file(self, |file| {
            file.as_ref()
                .seek(pos)
                .inspect_err(|e| error!("seek to file failed: {:?}", e))
                .map_err(|_| TEE_ERROR_GENERIC)
        })
    }

    fn ftruncate(&mut self, len: usize) -> TeeResult<()> {
        with_file(self, |file| {
            file.as_ref()
                .access(FileFlags::WRITE)
                .inspect_err(|e| error!("access file failed: {:?}", e))
                .map_err(|_| TEE_ERROR_GENERIC)?
                .set_len(len as _)
                .inspect_err(|e| error!("set len failed: {:?}", e))
                .map_err(|_| TEE_ERROR_GENERIC)
        })
    }

    fn close(&mut self) -> TeeResult<()> {
        if self.fd < 0 {
            return Ok(()); // already closed
        }
        TEE_FD_TABLE
            .write()
            .try_remove(self.fd as usize)
            .ok_or_else(|| {
                error!("remove file from fd table failed: {:?}", self.fd);
                TEE_ERROR_GENERIC
            })?;
        self.fd = -1;
        Ok(())
    }
}

pub fn tee_get_file_size(path: &str) -> TeeResult<usize> {
    let loc = resolve_at(AT_FDCWD, Some(path), 0)
        .inspect_err(|e| error!("resolve_at failed: {:?}", e))
        .map_err(|_| TEE_ERROR_GENERIC)?;
    Ok(loc.stat().map_err(|_| TEE_ERROR_GENERIC)?.size as usize)
}

pub fn file_ops_test() {
    let mut fd = FileVariant::open("/tmp/test.txt", (O_RDWR | O_CREAT) as u32, 0o644);
    assert!(fd.is_ok());
    let mut fd = fd.unwrap();

    // // write 1024 bytes to file
    let mut buf = [0xAA; 8];
    let n = fd.write(&buf).expect("Failed to write file");
}

#[cfg(feature = "tee_test")]
pub mod tests_file_ops {
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

        fn test_file_ops_read() {
            let mut fd = FileVariant::open("/tmp/test.txt", (O_RDWR | O_CREAT) as u32, 0o644);
            assert!(fd.is_ok());
            let mut fd = fd.unwrap();
            // // write 1024 bytes to file
            let mut buf = [0xAA; 8];
            let n = fd.write(&buf).expect("Failed to write file");
            assert_eq!(n, 8);
            // seek to 4 bytes from the beginning
            let pos = fd.seek(SeekFrom::Start(0)).expect("Failed to seek");
            assert_eq!(pos, 0);
            // // read 1024 bytes from file
            let mut buf = [0; 8];
            let n = fd.read(&mut buf).expect("Failed to read file");
            assert_eq!(n, 8);
            assert_eq!(buf, [0xAA; 8]);
            // pread 4 bytes from file at offset 4
            let mut buf = [0; 4];
            let n = fd.pread(&mut buf, 4).expect("Failed to pread file");
            assert_eq!(n, 4);
            assert_eq!(buf, [0xAA; 4]);
            // pwrite 4 bytes to file at offset 4
            let n = fd.pwrite(&[0xBB; 4], 4).expect("Failed to pwrite file");
            assert_eq!(n, 4);
            // read 4 bytes from file at offset 4
            let mut buf = [0; 4];
            let n = fd.pread(&mut buf, 4).expect("Failed to pread file");
            assert_eq!(n, 4);
            assert_eq!(buf, [0xBB; 4]);
            // truncate file to 4 bytes
            let n = fd.ftruncate(4).expect("Failed to truncate file");
            assert_eq!(n, ());
            // get file size
            let size = tee_get_file_size("/tmp/test.txt").expect("Failed to get file size");
            assert_eq!(size, 4);
        }
    }

    test_fn! {
        using TestResult;

        fn test_file_ops_exists() {
            let path = "/tmp/test.txt.not_exists";
            assert!(!FileVariant::exists(path));
            // create file
            {
                let mut fd = FileVariant::open(path, (O_RDWR | O_CREAT) as u32, 0o644);
                assert!(fd.is_ok());
            }
            // check if file exists
            assert!(FileVariant::exists(path));
            // remove file
            {
                let n = FileVariant::remove(path);
                assert!(n.is_ok());
            }
            // check if file exists
            assert!(!FileVariant::exists(path));
            // // remove file again
            // {
            //     let n = FileVariant::remove(path);
            //     assert!(n.is_ok());
            // }
        }
    }
    tests_name! {
        TEST_FILE_OPS;
        //------------------------
        test_file_ops_read,
        test_file_ops_exists,
    }
}
