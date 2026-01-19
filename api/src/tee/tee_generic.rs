use alloc::{format, vec::Vec};
use core::ffi::c_char;

use axnet::{
    SendOptions, SocketAddrEx, SocketOps,
    unix::{StreamTransport, UnixSocket, UnixSocketAddr},
};
use axtask::current;
use bincode::config;
use starry_core::task::AsThread;
use tee_raw_sys::{TEE_ERROR_BAD_PARAMETERS, TEE_ERROR_GENERIC};

use crate::{
    mm::vm_load_any_string,
    tee::{TeeResult, protocal::TeeRequest, tee_session::with_tee_ta_ctx},
};

pub fn sys_tee_scn_return(_return_code: u32) -> TeeResult {
    // Now we just ignore the return code and return Ok
    Ok(())
}

pub fn sys_tee_scn_log(buf: *const c_char, len: usize) -> TeeResult {
    // Implementation for TEE log syscall we use info to output the log now
    info!("TEE log syscall invoked with len: {}", len);
    let message = match vm_load_any_string(buf, len) {
        Ok(s) => s,
        Err(_) => return Err(TEE_ERROR_BAD_PARAMETERS),
    };

    info!("TEE Log: {}", message);

    Ok(())
}

pub fn sys_tee_scn_panic(panic_code: u32) -> TeeResult {
    // Connect to current TA via Unix socket
    let socket = UnixSocket::new(StreamTransport::new(
        current().as_thread().proc_data.proc.pid(),
    ));
    let uuid = with_tee_ta_ctx(|ctx| Ok(ctx.uuid.clone()))?;
    let path = format!("/tmp/{}.sock", uuid);
    let remote_addr = SocketAddrEx::Unix(UnixSocketAddr::Path(path.into()));
    socket.connect(remote_addr).map_err(|_| TEE_ERROR_GENERIC)?;

    // Send panic command request to current TA
    let req = TeeRequest::Panic { panic_code };
    let encoded = bincode::encode_to_vec(req, config::standard()).map_err(|_| TEE_ERROR_GENERIC)?;
    let mut message = Vec::with_capacity(4 + encoded.len());
    message.extend_from_slice(&(encoded.len() as u32).to_ne_bytes());
    message.extend_from_slice(&encoded);
    let mut src = message.as_slice();
    socket
        .send(&mut src, SendOptions::default())
        .map_err(|_| TEE_ERROR_GENERIC)?;
    Ok(())
}
