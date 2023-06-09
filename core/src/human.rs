use core::marker::PhantomData;

use crate::vm::Token;
use nom::error::{ErrorKind, ParseError};
use nom::IResult;

type Span<'a> = nom_locate::LocatedSpan<&'a str>;

/// Consumes a slash-slash-comment-eol, transforming it into the empty string.
fn eol_comment<'a, E: ParseError<Span<'a>>>(i: Span<'a>) -> IResult<Span<'a>, Span<'a>, E> {
    use nom::bytes::complete::{is_not, tag};
    use nom::sequence::preceded;
    preceded(tag("//"), is_not("\n\r"))(i)
}

/// Matches whitespace or eol comments across multiple lines.
fn ws_or_eol<'a, E: ParseError<Span<'a>>>(i: Span<'a>) -> IResult<Span<'a>, (), E> {
    use nom::branch::alt;
    use nom::character::complete::multispace1;
    use nom::multi::fold_many0;
    fold_many0(alt((eol_comment, multispace1)), || {}, |_, _| {})(i)
}

/// Consumes leading and trailing whitespace and comments, returning the output of `inner`.
fn ws<'a, F, O, E: ParseError<Span<'a>>>(
    inner: F,
) -> impl FnMut(Span<'a>) -> IResult<Span<'a>, O, E>
where
    F: FnMut(Span<'a>) -> IResult<Span<'a>, O, E>,
{
    use nom::sequence::delimited;
    delimited(ws_or_eol, inner, ws_or_eol)
}

fn word<'a, E: ParseError<Span<'a>>>(input: Span<'a>) -> IResult<Span<'a>, Token<Span<'a>>, E> {
    use nom::character::complete::alpha1;
    let (rem, all) = alpha1(input)?;
    Ok((rem, Token::Word(all)))
}

fn token<'a, E: ParseError<Span<'a>>>(input: Span<'a>) -> IResult<Span<'a>, Token<Span<'a>>, E> {
    use nom::branch::alt;
    alt((word,))(input)
}

pub struct Tokenizer<'a, E> {
    input: Span<'a>,
    done: bool,
    _mark: PhantomData<E>,
}
pub fn tokenize<'a, E: ParseError<Span<'a>>>(input: Span<'a>) -> Tokenizer<'a, E> {
    Tokenizer {
        input,
        done: false,
        _mark: PhantomData,
    }
}
impl<'a, E: ParseError<Span<'a>>> Iterator for Tokenizer<'a, E> {
    type Item = Result<Token<Span<'a>>, E>;
    fn next(&mut self) -> Option<Self::Item> {
        use nom::Err::*;
        if self.done {
            return None;
        }
        let res = match ws(token)(self.input) {
            Ok((remaining, token)) => {
                self.done = remaining.len() == 0;
                self.input = remaining;
                Ok(token)
            }
            Err(Incomplete(_)) => {
                self.done = true;
                Err(E::from_error_kind(self.input, ErrorKind::Eof))
            }
            Err(Error((rem, e))) | Err(Failure((rem, e))) => {
                if rem.len() == 0 {
                    return None;
                }
                self.done = true;
                Err(E::from_error_kind(rem, e))
            }
        };
        Some(res)
    }
}

#[cfg(test)]
mod test {
    extern crate std;
    use super::*;
    use nom::error::Error;
    use std::vec::Vec;

    #[test]
    fn word_test() {
        let word = word::<Error<&str>>;
        assert_eq!(word("hello"), Ok(("", Token::Word("hello"))));
    }

    #[test]
    fn ws_test() {
        let word = word::<Error<&str>>;
        assert_eq!(ws(word)("   hello"), Ok(("", Token::Word("hello"))));
        assert_eq!(ws(word)("hello   "), Ok(("", Token::Word("hello"))));
        assert_eq!(ws(word)("   hello   "), Ok(("", Token::Word("hello"))));
    }

    #[test]
    fn eol_comment_test() {
        let word = word::<Error<&str>>;
        const HELLO: Token<&str> = Token::Word("hello");
        assert_eq!(ws(word)("hello //test"), Ok(("", HELLO)));
        assert_eq!(ws(word)(" hello //test there"), Ok(("", HELLO)));
        assert_eq!(ws(word)(" //test\nhello"), Ok(("", HELLO)));
        assert_eq!(ws(word)("//test\nhello"), Ok(("", HELLO)));
        assert_eq!(ws(word)("//test\n//test\nhello"), Ok(("", HELLO)));
        assert_eq!(ws(word)("//test\n//test\n hello"), Ok(("", HELLO)));
    }

    #[test]
    fn tokenize_test() {
        let tokenize = |i| {
            let res: Vec<Result<Token<&str>, _>> = tokenize::<Error<&str>>(i).collect();
            res
        };
        assert_eq!(tokenize("hello //test"), [Ok(Token::Word("hello"))]);
        assert_eq!(
            tokenize("hello //test\n  world//test"),
            [Ok(Token::Word("hello")), Ok(Token::Word("world"))]
        );
    }
}
