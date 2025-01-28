use criterion::{criterion_group, criterion_main, Criterion};
use in_str::in_str;
use nom::{
  branch::alt,
  bytes::complete::{tag, take_while1, take_while_m_n},
  character::complete::char,
  combinator::opt,
  multi::many0_count,
  IResult, Parser,
};
use std::fs::read_to_string;
use whitehole::combinator::{eat, next};

fn lex_json_with_whitehole(s: &str) {
  // Use `* (1..)` to repeat for one or more times.
  let whitespaces = next(in_str!(" \t\r\n")) * (1..);

  let number = {
    let digit_1_to_9 = next(|c| matches!(c, '1'..='9'));
    // To re-use a combinator for multiple times, instead of wrapping the combinator in an Rc,
    // use a closure to generate the combinator for better runtime performance (via inlining).
    let digits = || next(|c| c.is_ascii_digit()) * (1..);
    let integer = eat('0') | (digit_1_to_9 + digits().optional());
    let fraction = eat('.') + digits();
    let exponent = (eat('e') | 'E') + (eat('-') | '+').optional() + digits();
    eat('-').optional() + integer + fraction.optional() + exponent.optional()
  };

  let string = {
    let body_optional = {
      let escape = {
        let simple = next(in_str!("\"\\/bfnrt"));
        let hex = eat('u') + next(|c| c.is_ascii_hexdigit()) * 4;
        eat('\\') + (simple | hex)
      };
      let non_escape =
        next(|c| c != '"' && c != '\\' && matches!(c, '\u{0020}'..='\u{10ffff}')) * (1..);

      // Use `* (..)` to repeat for zero or more times.
      (escape | non_escape) * ..
    };
    eat('"') + body_optional + '"'
  };

  let boundary = next(in_str!("[]{}:,"));

  let mut parser = whitehole::parser::Parser::builder()
    .entry(whitespaces | boundary | number | string | "true" | "false" | "null")
    .build(s);

  loop {
    let output = parser.parse();
    if output.is_none() {
      break;
    }
    // println!("{:?}", output);
  }

  if !parser.instant().rest().is_empty() {
    panic!(
      "lexer failed to consume the whole input, remaining: {:?}",
      &parser.instant().rest()[..100.min(parser.instant().rest().len())]
    );
  }
}

fn lex_json_with_nom(s: &str) {
  fn whitespaces(i: &str) -> IResult<&str, ()> {
    take_while1(in_str!(" \t\r\n")).map(|_| ()).parse(i)
  }

  fn number(i: &str) -> IResult<&str, ()> {
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

  fn string(i: &str) -> IResult<&str, ()> {
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

  fn boundary(i: &str) -> IResult<&str, ()> {
    alt((
      char('['),
      char(']'),
      char('{'),
      char('}'),
      char(':'),
      char(','),
    ))
    .map(|_| ())
    .parse(i)
  }

  fn entry(i: &str) -> IResult<&str, ()> {
    alt((
      whitespaces,
      boundary,
      number,
      string,
      tag("true").map(|_| ()),
      tag("false").map(|_| ()),
      tag("null").map(|_| ()),
    ))
    .parse(i)
  }

  let mut last_len = 0;
  let mut i = s;
  loop {
    i = entry(i).unwrap().0;
    if i.len() == 0 {
      break;
    }
    if i.len() == last_len {
      panic!(
        "lexer failed to consume the whole input, remaining: {:?}",
        &i[..100.min(i.len())]
      );
    }
    last_len = i.len();
  }
}

fn bench_lex(c: &mut Criterion) {
  // json files are from https://github.com/miloyip/nativejson-benchmark/tree/478d5727c2a4048e835a29c65adecc7d795360d5/data
  // you may need to download them manually
  let citm_catalog = read_to_string("bench_data/citm_catalog.json").unwrap();
  let twitter = read_to_string("bench_data/twitter.json").unwrap();
  let canada = read_to_string("bench_data/canada.json").unwrap();

  let total_bytes = citm_catalog.len() + twitter.len() + canada.len();

  c.bench_function(
    &format!(
      "lex_json_with_whitehole: lex 3 json files (total {} bytes)",
      total_bytes
    ),
    |b| {
      b.iter(|| {
        lex_json_with_whitehole(&citm_catalog);
        lex_json_with_whitehole(&twitter);
        lex_json_with_whitehole(&canada);
      })
    },
  );

  c.bench_function(
    &format!(
      "lex_json_with_nom: lex 3 json files (total {} bytes)",
      total_bytes
    ),
    |b| {
      b.iter(|| {
        lex_json_with_nom(&citm_catalog);
        lex_json_with_nom(&twitter);
        lex_json_with_nom(&canada);
      })
    },
  );
}

criterion_group! {
  name = benches;
  config = Criterion::default();
  targets = bench_lex
}
criterion_main!(benches);
