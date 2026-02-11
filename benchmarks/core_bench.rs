use aether_core::{ProbeManager, TargetInfo};
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_probe_listing(c: &mut Criterion) {
    let manager = ProbeManager::new();
    c.bench_function("probe_enumeration", |b| {
        b.iter(|| {
            let _ = black_box(manager.list_probes());
        })
    });
}

fn bench_target_info_allocation(c: &mut Criterion) {
    c.bench_function("target_info_creation", |b| {
        b.iter(|| {
            let _ = black_box(TargetInfo {
                name: "STM32F429ZITx".to_string(),
                flash_size: 2048 * 1024,
                ram_size: 256 * 1024,
                architecture: "Armv7em".to_string(),
            });
        })
    });
}

criterion_group!(benches, bench_probe_listing, bench_target_info_allocation);
criterion_main!(benches);
