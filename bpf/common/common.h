/* SPDX-License-Identifier: GPL-2.0 */
#pragma once

#include <linux/bpf.h>
#include <linux/if_ether.h>
#include <linux/ip.h>
#include <linux/tcp.h>
#include <linux/udp.h>
#include <bpf/bpf_helpers.h>
#include <bpf/bpf_endian.h>

/*
 * pod_meta — BPF map value carrying per-pod statistics.
 * Must match ebpf.PodMeta in internal/network/ebpf/maps.go.
 */
struct pod_meta {
    __u64 rx_packets;
    __u64 tx_packets;
    __u32 pod_ip;
    __u8  active;
    __u8  pad[3];
};

/*
 * svc_key — BPF map key for service load-balancing.
 * Must match ebpf.svcMapKey in internal/network/ebpf/maps.go.
 */
struct svc_key {
    __u32 vip;
    __u16 port;
    __u8  proto;
    __u8  pad;
};

/*
 * svc_backend — BPF map value for a single service backend.
 * Must match ebpf.Backend in internal/network/ebpf/maps.go.
 */
struct svc_backend {
    __u32 ip;
    __u16 port;
    __u16 pad;
};

/*
 * policy_key — BPF map key for network policy entries.
 * Must match ebpf.policyMapKey in internal/network/ebpf/maps.go.
 */
struct policy_key {
    __u32 src_ip;
    __u32 dst_ip;
};

/*
 * All BPF programs in Orchestra are GPL-licensed to allow use of
 * GPL-only kernel helpers.
 */
char LICENSE[] SEC("license") = "GPL";
