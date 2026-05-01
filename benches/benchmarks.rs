use {
  criterion::{BatchSize, Criterion, criterion_group, criterion_main},
  indoc::indoc,
  ruff_python_parser::{Mode, parse},
  std::hint::black_box,
  taipan::{Compiler, Machine},
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

fn bench_compile(c: &mut Criterion) {
  let mut group = c.benchmark_group("compile");

  for workload in WORKLOADS {
    let module = parse(workload.source, Mode::Module.into())
      .unwrap()
      .try_into_module()
      .unwrap();

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
      b.iter(|| {
        let module = parse(black_box(workload.source), Mode::Module.into())
          .unwrap()
          .try_into_module()
          .unwrap();

        let code = Compiler::compile(module.syntax()).unwrap();

        black_box(Machine::with_output(code, Vec::new()).unwrap());
      });
    });
  }

  group.finish();
}

fn bench_execute(c: &mut Criterion) {
  let mut group = c.benchmark_group("execute");

  for workload in WORKLOADS {
    let module = parse(workload.source, Mode::Module.into())
      .unwrap()
      .try_into_module()
      .unwrap();

    let code = Compiler::compile(module.syntax()).unwrap();

    group.bench_function(workload.name, |b| {
      b.iter_batched(
        || code.clone(),
        |code| Machine::with_output(code, Vec::new()).unwrap(),
        BatchSize::SmallInput,
      );
    });
  }

  group.finish();
}

fn bench_parse(c: &mut Criterion) {
  let mut group = c.benchmark_group("parse");

  for workload in WORKLOADS {
    group.bench_function(workload.name, |b| {
      b.iter(|| {
        black_box(
          parse(black_box(workload.source), Mode::Module.into())
            .unwrap()
            .try_into_module()
            .unwrap(),
        );
      });
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
