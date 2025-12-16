// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2025 KylinSoft Co., Ltd. <https://www.kylinos.cn/>
// See LICENSES for license details.
//
// This file has been created by KylinSoft on 2025.

use alloc::{format, string::ToString, vec::Vec};
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
        protocol::{Parameters, TARequest},
    },
};

const SERVER_SOCKET_PATH: &str = "/tmp/server.sock";

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
