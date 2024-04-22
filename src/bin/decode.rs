use std::io::{stdin, stdout, BufRead, BufWriter, Result, Write};

use lz::{
    decoder::Decoder,
    token::{DecoderAdapter, Token},
};

fn main() -> Result<()> {
    let reader = stdin().lock();
    let mut writer = BufWriter::new(stdout());
    let iter = reader
        .lines()
        .map(|line| Token::from(line.unwrap().parse::<u16>().unwrap()));
    let adapter = DecoderAdapter::new(iter);
    let decoder = Decoder::new(adapter);
    for x in decoder {
        writer.write_all(&[x])?;
    }
    Ok(())
}
