// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2025 WeiKang Guo <guoweikang.kernel@gmail.com
// Copyright (C) 2025 KylinSoft Co., Ltd. <https://www.kylinos.cn/>
// See LICENSE for license details.

//! Linux kernel chosen nodes

use crate::node::FdtNode;

/// Represents the node with interrupt-controller property
#[derive(Debug, Clone, Copy)]
pub struct InterruptController<'b, 'a> {
    pub(crate) node: FdtNode<'b, 'a>,
}

impl<'b, 'a: 'b> InterruptController<'b, 'a> {
    /// returns compatible property
    pub fn compatible(self) -> Option<&'a str> {
        match self.node.compatible() {
            Some(comp) => Some(comp.first()),
            None => None,
        }
    }

}
