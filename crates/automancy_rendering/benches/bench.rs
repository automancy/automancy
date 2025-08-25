use criterion::{criterion_group, criterion_main};

mod instance;

criterion_group!(
    benches,
    instance::bench_instance_manager_reference,
    instance::bench_instance_manager_insertion_clear_changes,
    instance::bench_instance_manager_insertion_clear_state,
    instance::bench_instance_manager_insertion,
);
criterion_main!(benches);
