﻿use super::_256;
use crate::{DataBlock, Quantize};

#[repr(C)]
pub struct IQ1M {
    pub qs: [u8; _256 / 8],
    pub qh: [u8; _256 / 16],
    pub scales: [u8; _256 / 32],
}

impl_data_block! {
    IQ1M = crate::types::IQ1M;
    Self {
        qs: [0; _256 / 8],
        qh: [0; _256 / 16],
        scales: [0; _256 / 32],
    }
}

impl Quantize<f32, _256> for IQ1M {
    fn quantize(_data: &[f32; _256]) -> Self {
        todo!()
    }
    fn dequantize(&self) -> [f32; _256] {
        todo!()
    }
}
