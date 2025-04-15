﻿use super::_256;
use crate::{DataBlock, Quantize};
use half::f16;

#[repr(C)]
pub struct Q5K {
    pub delta: f16,
    pub min: f16,
    pub scales: [u8; 12],
    pub qh: [u8; _256 / 8],
    pub qs: [u8; _256 / 2],
}

impl_data_block! {
    Q5K = crate::types::Q5K;
    Self {
        delta: f16::ZERO,
        min: f16::ZERO,
        scales: [0; 12],
        qh: [0; _256 / 8],
        qs: [0; _256 / 2],
    }
}

impl Quantize<f32, _256> for Q5K {
    fn quantize(_data: &[f32; _256]) -> Self {
        todo!()
    }
    fn dequantize(&self) -> [f32; _256] {
        todo!()
    }
}
