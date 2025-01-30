use criterion::{criterion_group, criterion_main, Criterion};
use in_str::in_str;
use nom::{branch::alt, bytes::complete::tag, character::complete::char, IResult, Parser};
use std::fs::read_to_string;
use whitehole::combinator::next;
use whitehole_bench::json::{number, number_nom, string, string_nom, whitespaces, whitespaces_nom};

fn lex_json_with_whitehole(s: &str) {
  let boundary = next(in_str!("[]{}:,"));

  let mut parser = whitehole::parser::Parser::builder()
    .entry(whitespaces() | boundary | number() | string() | "true" | "false" | "null")
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
      whitespaces_nom,
      boundary,
      number_nom,
      string_nom,
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
