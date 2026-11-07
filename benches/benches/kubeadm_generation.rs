use criterion::{Criterion, black_box, criterion_group, criterion_main};
use orchestra_core::cluster::ClusterSpec;
use orchestra_kubernetes::{generate_init_config, generate_join_config, rewrite_server_endpoint};

fn bench_kubeadm_manifest_generation(c: &mut Criterion) {
    let spec = ClusterSpec::new("bench-k8s", "v1.36.6");

    c.bench_function("generate_kubeadm_init_config", |b| {
        b.iter(|| {
            let yaml = generate_init_config(black_box(&spec), black_box("172.18.0.2")).unwrap();
            black_box(yaml);
        })
    });

    c.bench_function("generate_kubeadm_join_config", |b| {
        b.iter(|| {
            let yaml = generate_join_config(
                black_box("172.18.0.2:6443"),
                black_box("abcdef.0123456789abcdef"),
                black_box("sha256:1234567890abcdef"),
            );
            black_box(yaml);
        })
    });
}

fn bench_kubeconfig_rewriting(c: &mut Criterion) {
    let raw_kubeconfig = r#"
apiVersion: v1
clusters:
- cluster:
    certificate-authority-data: dGVzdC1jYQ==
    server: https://172.18.0.2:6443
  name: test-cluster
contexts:
- context:
    cluster: test-cluster
    user: kubernetes-admin
  name: kubernetes-admin@test-cluster
current-context: kubernetes-admin@test-cluster
kind: Config
users:
- name: kubernetes-admin
  user:
    client-certificate-data: dGVzdC1jZXJ0
    client-key-data: dGVzdC1rZXk=
"#;

    c.bench_function("rewrite_kubeconfig_endpoint", |b| {
        b.iter(|| {
            let rewritten =
                rewrite_server_endpoint(black_box(raw_kubeconfig), black_box(32800)).unwrap();
            black_box(rewritten);
        })
    });
}

criterion_group!(
    benches,
    bench_kubeadm_manifest_generation,
    bench_kubeconfig_rewriting
);
criterion_main!(benches);
