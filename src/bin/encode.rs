use std::io::{stdin, stdout, BufWriter, Result, Write};

use lz::{encoder::Encoder, token::EncoderAdaptor};

fn main() -> Result<()> {
    let mut writer = BufWriter::new(stdout());
    let encoder = Encoder::new(stdin(), false);
    let adaptor = EncoderAdaptor::new(encoder);
    for token in adaptor {
        writer.write_all(format!("{}", u16::from(token)).as_bytes())?;
    }
    Ok(())
}
