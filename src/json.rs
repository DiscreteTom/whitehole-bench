use in_str::in_str;
use nom::{
  branch::alt,
  bytes::complete::{take_while1, take_while_m_n},
  character::complete::char,
  combinator::opt,
  multi::many0_count,
  IResult, Parser,
};
use whitehole::{
  action::Action,
  combinator::{eat, next, Combinator},
};

pub fn whitespaces<State, Heap>() -> Combinator<impl Action<str, State, Heap, Value = ()>> {
  next(in_str!(" \t\r\n")) * (1..)
}

pub fn whitespaces_nom(i: &str) -> IResult<&str, ()> {
  take_while1(in_str!(" \t\r\n")).map(|_| ()).parse(i)
}

pub fn number<State, Heap>() -> Combinator<impl Action<str, State, Heap, Value = ()>> {
  let digit_1_to_9 = next(|c| matches!(c, '1'..='9'));
  let digits = || next(|c| c.is_ascii_digit()) * (1..);
  let integer = eat('0') | (digit_1_to_9 + digits().optional());
  let fraction = eat('.') + digits();
  let exponent = (eat('e') | 'E') + (eat('-') | '+').optional() + digits();
  eat('-').optional() + integer + fraction.optional() + exponent.optional()
}

pub fn number_nom(i: &str) -> IResult<&str, ()> {
  fn digits(i: &str) -> IResult<&str, ()> {
    take_while1(|c: char| c.is_ascii_digit())
      .map(|_| ())
      .parse(i)
  }
  fn integer(i: &str) -> IResult<&str, ()> {
    alt((char('0').map(|_| ()), digits.map(|_| ()))).parse(i)
  }
  fn fraction(i: &str) -> IResult<&str, ()> {
    let (i, _) = char('.')(i)?;
    digits(i)
  }
  fn exponent(i: &str) -> IResult<&str, ()> {
    let (i, _) = alt((char('e'), char('E'))).parse(i)?;
    let (i, _) = opt(alt((char('-'), char('+')))).parse(i)?;
    digits(i)
  }

  let (i, _) = opt(char('-')).parse(i)?;
  let (i, _) = integer(i)?;
  let (i, _) = opt(fraction).parse(i)?;
  opt(exponent).map(|_| ()).parse(i)
}

pub fn string<State, Heap>() -> Combinator<impl Action<str, State, Heap, Value = ()>> {
  let body_optional = {
    let escape = {
      let simple = next(in_str!("\"\\/bfnrt"));
      let hex = eat('u') + next(|c| c.is_ascii_hexdigit()) * 4;
      eat('\\') + (simple | hex)
    };

    let non_escape =
      next(|c| c != '"' && c != '\\' && matches!(c, '\u{0020}'..='\u{10ffff}')) * (1..);

    (escape | non_escape) * ..
  };
  eat('"') + body_optional + '"'
}

pub fn string_nom(i: &str) -> IResult<&str, ()> {
  fn body_optional(i: &str) -> IResult<&str, ()> {
    fn escape(i: &str) -> IResult<&str, ()> {
      fn simple(i: &str) -> IResult<&str, ()> {
        alt((
          char('"'),
          char('\\'),
          char('/'),
          char('b'),
          char('f'),
          char('n'),
          char('r'),
          char('t'),
        ))
        .map(|_| ())
        .parse(i)
      }
      fn hex(i: &str) -> IResult<&str, ()> {
        let (i, _) = char('u')(i)?;
        take_while_m_n(4, 4, |c: char| c.is_ascii_hexdigit())
          .map(|_| ())
          .parse(i)
      }

      let (i, _) = char('\\')(i)?;
      alt((simple, hex)).parse(i)
    }

    fn non_escape(i: &str) -> IResult<&str, ()> {
      take_while1(|c: char| c != '"' && c != '\\' && matches!(c, '\u{0020}'..='\u{10ffff}'))
        .map(|_| ())
        .parse(i)
    }

    many0_count(alt((escape, non_escape))).map(|_| ()).parse(i)
  }

  let (i, _) = char('"')(i)?;
  let (i, _) = body_optional(i)?;
  char('"').map(|_| ()).parse(i)
}
