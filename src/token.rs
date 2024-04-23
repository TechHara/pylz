use crate::lz77::Lz77;

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub enum MetaToken {
    StartOfSentence,
    EndOfSentence,
    Pad,
}

impl From<MetaToken> for u16 {
    fn from(token: MetaToken) -> Self {
        match token {
            MetaToken::StartOfSentence => 0,
            MetaToken::EndOfSentence => 1,
            MetaToken::Pad => 2,
        }
    }
}

impl From<u16> for MetaToken {
    fn from(value: u16) -> Self {
        match value {
            0 => MetaToken::StartOfSentence,
            1 => MetaToken::EndOfSentence,
            2 => MetaToken::Pad,
            _ => unreachable!(),
        }
    }
}

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub enum Token {
    Literal(u8),
    Length(u8), // 0~255 <-> 3~258
    // let d = distance - 1; d <-> 0~2^15-1
    Distance0(u8), // 0~255
    Distance1(u8), // 0~127 <-> 256 x (0~127)
    Meta(MetaToken),
}

impl From<Token> for u16 {
    fn from(token: Token) -> Self {
        match token {
            Token::Literal(x) => x as u16,
            Token::Length(l) => 256 + l as u16,
            Token::Distance0(d) => 512 + d as u16,
            Token::Distance1(d) => {
                debug_assert!(d <= 128);
                768 + d as u16
            }
            Token::Meta(t) => 896 + u16::from(t),
        }
    }
}

impl From<u16> for Token {
    fn from(x: u16) -> Self {
        match x {
            0..=255 => Token::Literal(x as u8),
            256..=511 => Token::Length((x & 0xFF) as u8),
            512..=767 => Token::Distance0((x & 0xFF) as u8),
            768..=895 => Token::Distance1((x & 0x7F) as u8),
            896..=898 => Token::Meta(MetaToken::from(x - 896)),
            _ => unreachable!(),
        }
    }
}

pub struct EncoderAdaptor<I> {
    iter: I,
    queue: Vec<Token>,
}

impl<I> EncoderAdaptor<I> {
    pub fn new(iter: I) -> Self {
        Self {
            iter,
            queue: Vec::with_capacity(2),
        }
    }
}

impl<I: Iterator<Item = Lz77>> Iterator for EncoderAdaptor<I> {
    type Item = Token;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(token) = self.queue.pop() {
            return Some(token);
        }

        let token = match self.iter.next()? {
            Lz77::Literal(x) => Token::Literal(x),
            Lz77::Dictionary { length, distance } => {
                let token = Token::Length(length);
                // queue distance tokens in reverse order
                let distance = distance - 1;
                debug_assert!(distance < 1 << 15);
                self.queue
                    .push(Token::Distance1(((distance & 0x7F00) >> 8) as u8));
                self.queue.push(Token::Distance0((distance & 0xFF) as u8));
                token
            }
        };
        Some(token)
    }
}

pub struct DecoderAdapter<I> {
    iter: I,
}

impl<I> DecoderAdapter<I> {
    pub fn new(iter: I) -> Self {
        Self { iter }
    }
}

impl<I: Iterator<Item = Token>> Iterator for DecoderAdapter<I> {
    type Item = Lz77;

    fn next(&mut self) -> Option<Self::Item> {
        let code = match self.iter.next()? {
            Token::Literal(x) => Lz77::Literal(x),
            Token::Length(l) => {
                let d0 = match self.iter.next() {
                    Some(Token::Distance0(x)) => x,
                    _ => unreachable!(),
                };
                let d1 = match self.iter.next() {
                    Some(Token::Distance1(x)) => x,
                    _ => unreachable!(),
                };
                let distance = (d0 as u16 | (d1 as u16) << 8) + 1;
                Lz77::Dictionary {
                    length: l,
                    distance,
                }
            }
            Token::Distance0(_) => unreachable!(),
            Token::Distance1(_) => unreachable!(),
            Token::Meta(_) => unreachable!(),
        };

        Some(code)
    }
}
