use criterion::{Criterion, black_box, criterion_group, criterion_main};
use orchestra_config::schema::OrchestraConfig;

fn bench_yaml_config_parsing(c: &mut Criterion) {
    let yaml = r#"
apiVersion: orchestra.io/v1alpha1
kind: Cluster
metadata:
  name: benchmark-cluster
  labels:
    env: staging
    region: us-east-1
spec:
  kubernetesVersion: v1.36.6
  nodes:
    controlPlane:
      count: 3
      image: orchestra-node:1.36.6
    workers:
      count: 10
      image: orchestra-node:1.36.6
  network:
    cni: kindnet
    podCidr: 10.244.0.0/16
    serviceCidr: 10.96.0.0/12
  proxy:
    mode: orchestra
"#;

    c.bench_function("parse_yaml_cluster_config", |b| {
        b.iter(|| {
            let config: OrchestraConfig = serde_yaml::from_str(black_box(yaml)).unwrap();
            black_box(config);
        })
    });
}

criterion_group!(benches, bench_yaml_config_parsing);
criterion_main!(benches);
