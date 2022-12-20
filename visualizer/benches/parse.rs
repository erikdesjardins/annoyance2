use criterion::{black_box, criterion_group, criterion_main, Criterion};
use visualizer::parse;
use visualizer::state::State;

fn benchmark(c: &mut Criterion) {
    let lines: Vec<_> = include_str!("./parse.txt").lines().collect();

    c.bench_function("handle_line cold", |b| {
        b.iter(|| {
            let mut state = State::default();
            for &line in &lines {
                black_box(parse::handle_line(&mut state, line));
            }
        })
    });

    c.bench_function("handle_line hot", |b| {
        // prefill the state, so we're not just testing allocation performance
        let mut state = State::default();
        for &line in &lines {
            black_box(parse::handle_line(&mut state, line));
        }

        b.iter(|| {
            for &line in &lines {
                black_box(parse::handle_line(&mut state, line));
            }
        })
    });
}

criterion_group!(benches, benchmark);
criterion_main!(benches);
