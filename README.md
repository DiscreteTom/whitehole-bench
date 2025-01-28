# whitehole-bench

Benchmark tests for [whitehole](https://github.com/DiscreteTom/whitehole).

## Usage

```bash
cargo bench
```

## Contribute

PR is welcome if you want to compare [whitehole](https://github.com/DiscreteTom/whitehole) with other libraries, in other scenarios, or you can improve the existing benchmark codes.

## Results

All the results are tested on my laptop. Just clone the repo and run `cargo bench` to get your own results.

### [JSON Lexer](./benches/json_lexer.rs)

```
lex_json_with_whitehole: lex 3 json files (total 4609769 bytes)
time:   [4.2659 ms 4.3519 ms 4.4500 ms]

lex_json_with_nom: lex 3 json files (total 4609769 bytes)
time:   [14.197 ms 14.456 ms 14.753 ms]
```
