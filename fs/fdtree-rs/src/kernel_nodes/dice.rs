// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2025 WeiKang Guo <guoweikang.kernel@gmail.com
// Copyright (C) 2025 KylinSoft Co., Ltd. <https://www.kylinos.cn/>
// See LICENSE for license details.

//! Kylin Dice

use crate::{
    node::FdtNode,
    standard_nodes::RegIter,
};

/// Represents the node with interrupt-controller property
#[derive(Debug, Clone, Copy)]
pub struct Dice<'b, 'a> {
    pub(crate) node: FdtNode<'b, 'a>,
}

impl<'b, 'a: 'b> Dice<'b, 'a> {
    /// Returns an iterator over all of the available memory regions
    pub fn regions(&self) -> Option<RegIter<'a>> {
        return self.node.reg()
    }
}
