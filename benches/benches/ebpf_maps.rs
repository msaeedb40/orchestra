use criterion::{Criterion, black_box, criterion_group, criterion_main};
use orchestra_ebpf::BpfMapManager;
use orchestra_ebpf_common::maps::{BackendValue, ConntrackEntry, ConntrackKey, ServiceKey};

fn bench_ebpf_map_lookups(c: &mut Criterion) {
    let manager = BpfMapManager::new();

    // Seed 1,000 services with 3 backends each
    for i in 0..1000 {
        let b3 = (i % 250) as u8;
        let b2 = (i / 250) as u8;
        let key = ServiceKey::new_ipv4([10, 96, b2, b3], 80, 6);
        let backends = vec![
            BackendValue::new_ipv4([10, 244, b2, b3], 8080, 100),
            BackendValue::new_ipv4([10, 244, b2, b3 + 1], 8080, 100),
            BackendValue::new_ipv4([10, 244, b2, b3 + 2], 8080, 100),
        ];
        manager.upsert_service(key, i as u32, 0, &backends).unwrap();
    }

    let lookup_key = ServiceKey::new_ipv4([10, 96, 1, 100], 80, 6);

    c.bench_function("bpf_map_service_lookup", |b| {
        b.iter(|| {
            let res = manager.get_service(black_box(&lookup_key));
            black_box(res);
        })
    });

    c.bench_function("bpf_map_backend_lookup", |b| {
        b.iter(|| {
            let res = manager.get_backend(black_box(350), black_box(1));
            black_box(res);
        })
    });
}

fn bench_conntrack_operations(c: &mut Criterion) {
    let manager = BpfMapManager::new();

    let ct_key = ConntrackKey {
        src_address: [10, 244, 1, 5, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        dst_address: [10, 96, 0, 10, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        src_port: 50123,
        dst_port: 443,
        protocol: 6,
        _pad: [0; 3],
    };

    let ct_entry = ConntrackEntry {
        original_dst_address: [10, 96, 0, 10, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        translated_dst_address: [10, 244, 1, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        original_dst_port: 443,
        translated_dst_port: 8443,
        state: 1,
        last_seen_ns: 12345678,
        packet_count: 1,
        byte_count: 64,
    };

    c.bench_function("bpf_conntrack_insert", |b| {
        b.iter(|| {
            manager.update_conntrack(black_box(ct_key), black_box(ct_entry));
        })
    });
}

criterion_group!(benches, bench_ebpf_map_lookups, bench_conntrack_operations);
criterion_main!(benches);
