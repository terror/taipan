use {
  criterion::{BatchSize, Criterion, criterion_group, criterion_main},
  indoc::indoc,
  ruff_python_ast::ModModule,
  ruff_python_parser::{Mode, Parsed, parse},
  std::hint::black_box,
  taipan::{Code, Compiler, Machine, Object},
};

struct Workload {
  name: &'static str,
  source: &'static str,
}

const WORKLOADS: &[Workload] = &[
  Workload {
    name: "while_loop",
    source: indoc! {
      "
      i = 0
      while i < 1000:
        i += 1
      "
    },
  },
  Workload {
    name: "arithmetic",
    source: indoc! {
      "
      i = 0
      x = 1
      while i < 1000:
        x = x + i
        x = x * 3
        x = x % 1000003
        x = x - i
        i += 1
      "
    },
  },
  Workload {
    name: "function_calls",
    source: indoc! {
      "
      def foo(x):
        return x + 1

      i = 0
      x = 0
      while i < 500:
        x = foo(x)
        i += 1
      "
    },
  },
  Workload {
    name: "global_builtin_lookup",
    source: indoc! {
      r#"
      i = 0
      x = 0
      while i < 500:
        x += len("foo")
        x += int("1")
        x += abs(-1)
        i += 1
      "#
    },
  },
  Workload {
    name: "string_concatenation",
    source: indoc! {
      r#"
      i = 0
      s = ""
      while i < 200:
        s += "foo"
        i += 1
      "#
    },
  },
  Workload {
    name: "printing",
    source: indoc! {
      r#"
      i = 0
      while i < 100:
        print("foo", i)
        i += 1
      "#
    },
  },
];

fn compile(source: &str) -> Code {
  Compiler::compile(parse_module(source).syntax()).unwrap()
}

fn end_to_end(source: &str) -> (Object, Vec<u8>) {
  run(compile(source))
}

fn parse_module(source: &str) -> Parsed<ModModule> {
  parse(source, Mode::Module.into())
    .unwrap()
    .try_into_module()
    .unwrap()
}

fn run(code: Code) -> (Object, Vec<u8>) {
  Machine::with_output(code, Vec::new()).unwrap()
}

fn bench_compile(c: &mut Criterion) {
  let mut group = c.benchmark_group("compile");

  for workload in WORKLOADS {
    let module = parse_module(workload.source);

    group.bench_function(workload.name, |b| {
      b.iter(|| Compiler::compile(black_box(module.syntax())).unwrap());
    });
  }

  group.finish();
}

fn bench_end_to_end(c: &mut Criterion) {
  let mut group = c.benchmark_group("end_to_end");

  for workload in WORKLOADS {
    group.bench_function(workload.name, |b| {
      b.iter(|| black_box(end_to_end(black_box(workload.source))));
    });
  }

  group.finish();
}

fn bench_execute(c: &mut Criterion) {
  let mut group = c.benchmark_group("execute");

  for workload in WORKLOADS {
    let code = compile(workload.source);

    group.bench_function(workload.name, |b| {
      b.iter_batched(|| code.clone(), run, BatchSize::SmallInput);
    });
  }

  group.finish();
}

fn bench_parse(c: &mut Criterion) {
  let mut group = c.benchmark_group("parse");

  for workload in WORKLOADS {
    group.bench_function(workload.name, |b| {
      b.iter(|| black_box(parse_module(black_box(workload.source))));
    });
  }

  group.finish();
}

criterion_group!(
  benches,
  bench_parse,
  bench_compile,
  bench_execute,
  bench_end_to_end
);
criterion_main!(benches);
