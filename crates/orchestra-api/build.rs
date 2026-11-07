fn main() -> Result<(), Box<dyn std::error::Error>> {
    let proto_dir = "../../proto";
    let protos = [
        "../../proto/orchestra/common/v1/common.proto",
        "../../proto/orchestra/cluster/v1/cluster.proto",
        "../../proto/orchestra/node/v1/node.proto",
        "../../proto/orchestra/network/v1/network.proto",
        "../../proto/orchestra/ebpf/v1/ebpf.proto",
        "../../proto/orchestra/system/v1/system.proto",
    ];

    println!("cargo:rerun-if-changed={proto_dir}");
    for proto in &protos {
        println!("cargo:rerun-if-changed={proto}");
    }

    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .compile_protos(&protos, &[proto_dir])?;

    Ok(())
}
