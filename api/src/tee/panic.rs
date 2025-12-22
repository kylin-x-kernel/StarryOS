use alloc::{format, vec::Vec};

use axnet::{
    SendOptions, SocketAddrEx, SocketOps,
    unix::{StreamTransport, UnixSocket, UnixSocketAddr},
};
use axtask::current;
use bincode::config;
use starry_core::task::AsThread;
use tee_raw_sys::TEE_ERROR_GENERIC;

use crate::tee::{TeeResult, protocol::CARequest, tee_session::with_tee_ta_ctx};

pub(crate) fn sys_tee_scn_panic(panic_code: u32) -> TeeResult {
    // Connect to current TA via Unix socket
    let socket = UnixSocket::new(StreamTransport::new(
        current().as_thread().proc_data.proc.pid(),
    ));
    let uuid = with_tee_ta_ctx(|ctx| Ok(ctx.uuid.clone()))?;
    let path = format!("/tmp/{}.sock", uuid);
    let remote_addr = SocketAddrEx::Unix(UnixSocketAddr::Path(path.into()));
    socket.connect(remote_addr).map_err(|_| TEE_ERROR_GENERIC)?;

    // Send destroy command request to current TA
    let req = CARequest::Destroy {};
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
