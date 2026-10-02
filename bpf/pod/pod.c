/* SPDX-License-Identifier: GPL-2.0 */
/*
 * pod.c — TC BPF programs for per-pod packet accounting.
 *
 * pod_ingress: counts packets destined for a known pod IP (rx_packets).
 * pod_egress:  counts packets originating from a known pod IP (tx_packets).
 *
 * Map key: big-endian IPv4 address (__u32).
 * Map value: struct pod_meta (see bpf/common/common.h).
 */
#include "../../bpf/common/common.h"

/*
 * pod_map — per-pod statistics hash map.
 *
 *  key   = IPv4 address of the pod (__u32, network byte order)
 *  value = struct pod_meta
 */
struct {
    __uint(type,       BPF_MAP_TYPE_HASH);
    __uint(max_entries, 65536);
    __type(key,        __u32);
    __type(value,      struct pod_meta);
} pod_map SEC(".maps");

/*
 * pod_ingress — TC ingress hook.
 *
 * Parses the Ethernet + IP headers with verifier-safe bounds checks, looks up
 * the destination address in pod_map, and increments rx_packets atomically
 * before passing the packet on.
 */
SEC("tc/ingress")
int pod_ingress(struct __sk_buff *skb)
{
    void *data     = (void *)(long)skb->data;
    void *data_end = (void *)(long)skb->data_end;

    /* Ethernet header bounds check */
    struct ethhdr *eth = data;
    if ((void *)(eth + 1) > data_end)
        return TC_ACT_OK;

    /* Only handle IPv4 */
    if (eth->h_proto != bpf_htons(ETH_P_IP))
        return TC_ACT_OK;

    /* IP header bounds check */
    struct iphdr *iph = (struct iphdr *)(eth + 1);
    if ((void *)(iph + 1) > data_end)
        return TC_ACT_OK;

    /* Look up the destination pod */
    __u32 daddr = iph->daddr;
    struct pod_meta *meta = bpf_map_lookup_elem(&pod_map, &daddr);
    if (!meta)
        return TC_ACT_OK;

    /* Atomically increment the ingress counter */
    __sync_fetch_and_add(&meta->rx_packets, 1);

    return TC_ACT_OK;
}

/*
 * pod_egress — TC egress hook.
 *
 * Parses Ethernet + IP headers, looks up the source address in pod_map, and
 * increments tx_packets atomically.
 */
SEC("tc/egress")
int pod_egress(struct __sk_buff *skb)
{
    void *data     = (void *)(long)skb->data;
    void *data_end = (void *)(long)skb->data_end;

    /* Ethernet header bounds check */
    struct ethhdr *eth = data;
    if ((void *)(eth + 1) > data_end)
        return TC_ACT_OK;

    /* Only handle IPv4 */
    if (eth->h_proto != bpf_htons(ETH_P_IP))
        return TC_ACT_OK;

    /* IP header bounds check */
    struct iphdr *iph = (struct iphdr *)(eth + 1);
    if ((void *)(iph + 1) > data_end)
        return TC_ACT_OK;

    /* Look up the source pod */
    __u32 saddr = iph->saddr;
    struct pod_meta *meta = bpf_map_lookup_elem(&pod_map, &saddr);
    if (!meta)
        return TC_ACT_OK;

    /* Atomically increment the egress counter */
    __sync_fetch_and_add(&meta->tx_packets, 1);

    return TC_ACT_OK;
}
