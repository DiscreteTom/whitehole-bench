use criterion::{criterion_group, criterion_main, Criterion};
use in_str::in_str;
use std::fs::read_to_string;
use whitehole::{
  combinator::{eat, next},
  parser::Parser,
};

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

  let mut parser = Parser::builder()
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
}

criterion_group! {
  name = benches;
  config = Criterion::default();
  targets = bench_lex
}
criterion_main!(benches);
