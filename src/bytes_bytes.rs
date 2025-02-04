//! Bytes trait implementations for the `bytes` crate

use crate::types::{Bytes, MutBytes, ResizableBytes};

impl Bytes for bytes::Bytes {
    fn as_slice(&self) -> &[u8] {
        self.as_ref()
    }

    fn len(&self) -> usize {
        self.len()
    }

    fn is_empty(&self) -> bool {
        self.is_empty()
    }
}

impl Bytes for bytes::BytesMut {
    fn as_slice(&self) -> &[u8] {
        self.as_ref()
    }

    fn len(&self) -> usize {
        self.len()
    }

    fn is_empty(&self) -> bool {
        self.is_empty()
    }
}

impl MutBytes for bytes::BytesMut {
    fn as_mut_slice(&mut self) -> &mut [u8] {
        self.as_mut()
    }

    fn copy_from_slice(&mut self, other: &[u8]) {
        self.as_mut().copy_from_slice(other)
    }
}

impl ResizableBytes for bytes::BytesMut {
    fn resize(&mut self, new_len: usize, value: u8) {
        self.resize(new_len, value)
    }
}
