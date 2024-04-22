pub mod decoder;
pub mod encoder;
pub mod hash;
pub mod hash_chain;
pub mod lz77;
pub mod token;

use std::io::Cursor;

use decoder::Decoder;
use encoder::Encoder;
use numpy::{PyArray1, PyReadonlyArrayDyn};
use pyo3::prelude::*;
use token::{DecoderAdapter, EncoderAdaptor, Token};

fn encode(xs: &[u8]) -> Vec<u16> {
    let cursor = Cursor::new(xs);
    let encoder = Encoder::new(cursor, false);
    let adaptor = EncoderAdaptor::new(encoder);
    adaptor.map(|token| u16::from(token)).collect()
}

fn decode(xs: &[u16]) -> Vec<u8> {
    let adaptor = DecoderAdapter::new(xs.iter().map(|x| Token::from(*x)));
    let decoder = Decoder::new(adaptor);
    decoder.collect()
}

/// A Python module implemented in Rust. The name of this function must match
/// the `lib.name` setting in the `Cargo.toml`, else Python will not be able to
/// import the module.
#[pymodule]
fn lz(m: &Bound<'_, PyModule>) -> PyResult<()> {
    #[pyfn(m)]
    #[pyo3(name = "encode")]
    fn encode_py<'py>(
        py: Python<'py>,
        xs: PyReadonlyArrayDyn<'py, u8>,
    ) -> Bound<'py, PyArray1<u16>> {
        let result = encode(xs.as_slice().unwrap());
        let array = PyArray1::from_vec_bound(py, result);
        array
    }

    #[pyfn(m)]
    #[pyo3(name = "decode")]
    fn decode_py<'py>(
        py: Python<'py>,
        xs: PyReadonlyArrayDyn<'py, u16>,
    ) -> Bound<'py, PyArray1<u8>> {
        let result = decode(xs.as_slice().unwrap());
        let array = PyArray1::from_vec_bound(py, result);
        array
    }

    Ok(())
}
