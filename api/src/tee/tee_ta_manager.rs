// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2025 KylinSoft Co., Ltd. <https://www.kylinos.cn/>
// See LICENSES for license details.
//
// This file has been created by KylinSoft on 2025.

use alloc::{
    format,
    string::{String, ToString},
    vec::Vec,
};
use axnet::{
    SendOptions, SocketAddrEx, SocketOps,
    unix::{StreamTransport, UnixSocket, UnixSocketAddr},
};
use axtask::current;
use bincode::config;
use starry_core::task::{AsThread, Thread};
use tee_raw_sys::{TEE_ERROR_GENERIC, TEE_ERROR_ITEM_NOT_FOUND, TEE_ErrorOrigin};

use crate::{
    socket::SocketAddrExt,
    tee::{
        TeeResult,
        protocol::{CARequest, Parameters, TARequest},
    },
};

const SERVER_SOCKET_PATH: &str = "/tmp/server.sock";

pub struct SessionIdentity {
    uuid: String,
    session_id: u32,
}

pub fn tee_ta_init_session(uuid: &str) -> TeeResult {
    // Connect to the vsock-manager via Unix socket
    let socket = UnixSocket::new(StreamTransport::new(
        current().as_thread().proc_data.proc.pid(),
    ));
    let remote_addr = SocketAddrEx::Unix(UnixSocketAddr::Path(SERVER_SOCKET_PATH.into()));
    socket.connect(remote_addr).map_err(|_| TEE_ERROR_GENERIC)?;

    // Send open session request to dest TA
    let req = TARequest::OpenSession {
        uuid: uuid.to_string(),
        params: Parameters::default(),
    };
    let encoded = bincode::encode_to_vec(req, config::standard()).map_err(|_| TEE_ERROR_GENERIC)?;
    let mut src = encoded.as_slice();
    socket
        .send(&mut src, SendOptions::default())
        .map_err(|_| TEE_ERROR_GENERIC)?;

    Ok(())
}

pub fn tee_ta_close_session(sess_id: SessionIdentity) -> TeeResult {
    // Connect to dest TA via Unix socket
    let socket = UnixSocket::new(StreamTransport::new(
        current().as_thread().proc_data.proc.pid(),
    ));
    let path = format!("/tmp/{}.sock", sess_id.uuid);
    let remote_addr = SocketAddrEx::Unix(UnixSocketAddr::Path(path.into()));
    socket.connect(remote_addr).map_err(|_| TEE_ERROR_GENERIC)?;

    // Send close session request to dest TA
    let req = CARequest::CloseSession {
        session_id: sess_id.session_id,
    };
    let encoded = bincode::encode_to_vec(req, config::standard()).map_err(|_| TEE_ERROR_GENERIC)?;
    let mut src = encoded.as_slice();
    socket
        .send(&mut src, SendOptions::default())
        .map_err(|_| TEE_ERROR_GENERIC)?;

    Ok(())
}

pub fn tee_ta_get_session(handle: u32) -> TeeResult<SessionIdentity> {
    todo!()
}
