use criterion::{Criterion, black_box, criterion_group, criterion_main};
use orchestra_common::utilities::parse_cidr;
use orchestra_network::ipam::IpamPool;

fn bench_ipam_allocation(c: &mut Criterion) {
    let pool = IpamPool::new("10.244.0.0/16").unwrap();

    c.bench_function("ipam_allocate_sequential", |b| {
        b.iter(|| {
            let ip = pool.allocate().unwrap();
            black_box(ip);
            pool.release(ip);
        })
    });
}

fn bench_cidr_containment(c: &mut Criterion) {
    let cidr = parse_cidr("10.244.0.0/16").unwrap();
    let ip: std::net::IpAddr = "10.244.15.200".parse().unwrap();

    c.bench_function("cidr_containment_check", |b| {
        b.iter(|| {
            let contains = cidr.contains(black_box(&ip));
            black_box(contains);
        })
    });
}

criterion_group!(benches, bench_ipam_allocation, bench_cidr_containment);
criterion_main!(benches);
