use criterion::{criterion_group, criterion_main, Criterion};
use nom::{
  branch::alt, bytes::complete::tag, character::complete::char, combinator::opt,
  multi::separated_list0, IResult, Parser,
};
use std::{collections::HashMap, fs::read_to_string};
use whitehole::{
  combinator::{eat, recur},
  range::Range,
};
use whitehole_bench::json::{number, number_nom, string, string_nom, whitespaces, whitespaces_nom};

enum Value {
  Null,
  Bool(bool),
  Number(f64),
  String(Range),
  Array(Vec<Value>),
  Object(HashMap<Range, Value>),
}

enum NomValue<'a> {
  Null,
  Bool(bool),
  Number(f64),
  String(&'a str),
  Array(Vec<NomValue<'a>>),
  Object(HashMap<&'a str, NomValue<'a>>),
}

fn parse_json_with_whitehole(s: &str) -> Value {
  let wso = || whitespaces().optional();
  let number = || number().select(|ctx| Value::Number(ctx.content().parse().unwrap()));
  let string = || string().select(|ctx| ctx.range());

  let (value, value_setter) = recur::<_, (), (), _>();

  let sep = || eat(',') + wso();

  let array = || {
    let values = ((value().tuple() + wso()).pop() * (..))
      .fold(
        || vec![],
        |mut acc, v| {
          acc.push(v);
          acc
        },
      )
      .sep(sep());

    (eat('[') + wso() + values.tuple() + ']')
      .pop()
      .map(|v| Value::Array(v))
  };

  let object = || {
    let entries = {
      let entry = string().tuple() + wso() + ':' + wso() + value().tuple();

      ((entry + wso()) * (..))
        .fold(
          || HashMap::new(),
          |mut acc, (k, v)| {
            acc.insert(k, v);
            acc
          },
        )
        .sep(sep())
    };

    (eat('{') + wso() + entries.tuple() + '}')
      .pop()
      .map(Value::Object)
  };

  value_setter.boxed(
    array()
      | object()
      | number()
      | string().map(Value::String)
      | eat("true").map::<_, (), (), _, _>(|_| Value::Bool(true))
      | eat("false").map::<_, (), (), _, _>(|_| Value::Bool(false))
      | eat("null").map::<_, (), (), _, _>(|_| Value::Null),
  );

  let mut parser = whitehole::parser::Parser::builder()
    .entry(whitespaces().map(|_| None) | value().map(Some))
    .build(s);

  let mut v = Value::Null;

  loop {
    match parser.parse() {
      None => break,
      Some(output) => {
        if let Some(value) = output.value {
          v = value;
        }
      }
    }
  }

  if !parser.instant().rest().is_empty() {
    panic!(
      "parser failed to consume the whole input, remaining: {:?}",
      &parser.instant().rest()[..100.min(parser.instant().rest().len())]
    );
  }

  if matches!(v, Value::Null) {
    panic!("parser failed to parse the input");
  }

  v
}

fn parse_json_with_nom(s: &str) -> NomValue {
  fn wso(i: &str) -> IResult<&str, ()> {
    opt(whitespaces_nom).map(|_| ()).parse(i)
  }

  fn number(i: &str) -> IResult<&str, NomValue> {
    let (rest, _) = number_nom(i)?;
    Ok((
      rest,
      NomValue::Number(i[..i.len() - rest.len()].parse().unwrap()),
    ))
  }

  fn string(i: &str) -> IResult<&str, &str> {
    let (rest, _) = string_nom(i)?;
    Ok((rest, &i[..i.len() - rest.len()]))
  }

  fn sep(i: &str) -> IResult<&str, ()> {
    let (i, _) = char(',')(i)?;
    wso(i)
  }

  fn array(i: &str) -> IResult<&str, NomValue> {
    fn values(i: &str) -> IResult<&str, Vec<NomValue>> {
      fn value_and_wso(i: &str) -> IResult<&str, NomValue> {
        let (i, value) = value(i)?;
        let (i, _) = wso(i)?;
        Ok((i, value))
      }

      separated_list0(sep, value_and_wso).parse(i)
    }

    let (i, _) = char('[')(i)?;
    let (i, _) = wso(i)?;
    let (i, v) = values(i)?;
    let (i, _) = char(']')(i)?;
    Ok((i, NomValue::Array(v)))
  }

  fn object(i: &str) -> IResult<&str, NomValue> {
    fn entries(i: &str) -> IResult<&str, NomValue> {
      fn entry_and_wso(i: &str) -> IResult<&str, (&str, NomValue)> {
        fn entry(i: &str) -> IResult<&str, (&str, NomValue)> {
          let (i, k) = string(i)?;
          let (i, _) = wso(i)?;
          let (i, _) = char(':')(i)?;
          let (i, _) = wso(i)?;
          let (i, v) = value(i)?;
          Ok((i, (k, v)))
        }
        let (i, v) = entry(i)?;
        let (i, _) = wso(i)?;
        Ok((i, v))
      }

      // TODO: how to use fold with separated_list0? currently collecting to a vec and then converting to a hashmap is inefficient
      separated_list0(sep, entry_and_wso)
        .map(|v| NomValue::Object(v.into_iter().collect()))
        .parse(i)
    }

    let (i, _) = char('{')(i)?;
    let (i, _) = wso(i)?;
    let (i, v) = entries(i)?;
    let (i, _) = char('}')(i)?;
    Ok((i, v))
  }

  fn value(i: &str) -> IResult<&str, NomValue> {
    alt((
      array,
      object,
      number,
      |i| string.map(NomValue::String).parse(i),
      |i| tag("true").map(|_| NomValue::Bool(true)).parse(i),
      |i| tag("false").map(|_| NomValue::Bool(false)).parse(i),
      |i| tag("null").map(|_| NomValue::Null).parse(i),
    ))
    .parse(i)
  }

  fn entry(i: &str) -> IResult<&str, Option<NomValue>> {
    alt((whitespaces_nom.map(|_| None), value.map(Some))).parse(i)
  }

  let mut last_len = 0;
  let mut v = NomValue::Null;
  let mut i = s;
  loop {
    let (new_i, new_v) = entry(i).unwrap();
    if let Some(new_v) = new_v {
      v = new_v;
    }

    i = new_i;
    if i.len() == 0 {
      break;
    }
    if i.len() == last_len {
      panic!(
        "parser failed to consume the whole input, remaining: {:?}",
        &i[..100.min(i.len())]
      );
    }
    last_len = i.len();
  }

  if matches!(v, NomValue::Null) {
    panic!("parser failed to parse the input");
  }

  v
}

fn bench_parse(c: &mut Criterion) {
  let citm_catalog = read_to_string("bench_data/citm_catalog.json").unwrap();
  let twitter = read_to_string("bench_data/twitter.json").unwrap();
  let canada = read_to_string("bench_data/canada.json").unwrap();

  let total_bytes = citm_catalog.len() + twitter.len() + canada.len();

  c.bench_function(
    &format!(
      "parse_json_with_whitehole: parse 3 json files (total {} bytes)",
      total_bytes
    ),
    |b| {
      b.iter(|| {
        parse_json_with_whitehole(&citm_catalog);
        parse_json_with_whitehole(&twitter);
        parse_json_with_whitehole(&canada);
      })
    },
  );

  c.bench_function(
    &format!(
      "parse_json_with_nom: parse 3 json files (total {} bytes)",
      total_bytes
    ),
    |b| {
      b.iter(|| {
        parse_json_with_nom(&citm_catalog);
        parse_json_with_nom(&twitter);
        parse_json_with_nom(&canada);
      })
    },
  );
}

criterion_group! {
  name = benches;
  config = Criterion::default();
  targets = bench_parse
}
criterion_main!(benches);
