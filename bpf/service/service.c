/* SPDX-License-Identifier: GPL-2.0 */
/*
 * service.c — TC BPF service load-balancer (DNAT).
 *
 * svc_lb: on ingress, matches the destination VIP + port against svc_map and
 * rewrites daddr + dport to the configured backend, then recalculates the IP
 * and transport-layer checksums so the packet is delivered correctly.
 *
 * Supports TCP and UDP.
 */
#include "../../bpf/common/common.h"

/*
 * svc_map — service VIP → single backend hash map.
 *
 *  key   = struct svc_key  { vip, port, proto }
 *  value = struct svc_backend { ip, port }
 */
struct {
    __uint(type,        BPF_MAP_TYPE_HASH);
    __uint(max_entries, 4096);
    __type(key,         struct svc_key);
    __type(value,       struct svc_backend);
} svc_map SEC(".maps");

/*
 * svc_lb — TC ingress hook.
 *
 * Parses ETH + IP + TCP/UDP headers with verifier-safe bounds checks.
 * If the packet matches a service VIP:port, it is DNAT-ed to the backend and
 * both the IP and transport-layer checksums are recomputed.
 */
SEC("tc/ingress")
int svc_lb(struct __sk_buff *skb)
{
    void *data     = (void *)(long)skb->data;
    void *data_end = (void *)(long)skb->data_end;

    /* ---- Ethernet ---- */
    struct ethhdr *eth = data;
    if ((void *)(eth + 1) > data_end)
        return TC_ACT_OK;

    if (eth->h_proto != bpf_htons(ETH_P_IP))
        return TC_ACT_OK;

    /* ---- IP ---- */
    struct iphdr *iph = (struct iphdr *)(eth + 1);
    if ((void *)(iph + 1) > data_end)
        return TC_ACT_OK;

    __u8 proto = iph->protocol;
    if (proto != IPPROTO_TCP && proto != IPPROTO_UDP)
        return TC_ACT_OK;

    /* Reject IP options to keep offset arithmetic simple */
    if (iph->ihl != 5)
        return TC_ACT_OK;

    /* ---- TCP / UDP destination port ---- */
    __u16 dport = 0;
    __u32 l4_off = sizeof(struct ethhdr) + sizeof(struct iphdr);

    if (proto == IPPROTO_TCP) {
        struct tcphdr *tcph = (struct tcphdr *)(iph + 1);
        if ((void *)(tcph + 1) > data_end)
            return TC_ACT_OK;
        dport = tcph->dest;
    } else {
        struct udphdr *udph = (struct udphdr *)(iph + 1);
        if ((void *)(udph + 1) > data_end)
            return TC_ACT_OK;
        dport = udph->dest;
    }

    /* ---- Service map lookup ---- */
    struct svc_key key = {
        .vip   = iph->daddr,
        .port  = dport,
        .proto = proto,
        .pad   = 0,
    };

    struct svc_backend *backend = bpf_map_lookup_elem(&svc_map, &key);
    if (!backend)
        return TC_ACT_OK;

    /* ---- Rewrite destination IP (DNAT) ---- */
    __u32 old_ip  = iph->daddr;
    __u32 new_ip  = backend->ip;
    __u16 old_port = dport;
    __u16 new_port = backend->port;

    /* Update IP destination — bpf_l3_csum_replace handles the IP checksum */
    bpf_l3_csum_replace(skb,
                        sizeof(struct ethhdr) + offsetof(struct iphdr, check),
                        old_ip, new_ip, sizeof(__u32));
    bpf_skb_store_bytes(skb,
                        sizeof(struct ethhdr) + offsetof(struct iphdr, daddr),
                        &new_ip, sizeof(new_ip), 0);

    /* ---- Rewrite destination port + transport checksum ---- */
    if (proto == IPPROTO_TCP) {
        bpf_l4_csum_replace(skb,
                            l4_off + offsetof(struct tcphdr, check),
                            old_ip, new_ip,
                            BPF_F_PSEUDO_HDR | sizeof(__u32));
        bpf_l4_csum_replace(skb,
                            l4_off + offsetof(struct tcphdr, check),
                            old_port, new_port,
                            sizeof(__u16));
        bpf_skb_store_bytes(skb,
                            l4_off + offsetof(struct tcphdr, dest),
                            &new_port, sizeof(new_port), 0);
    } else {
        /* UDP checksum is optional (0 = disabled); only update if non-zero */
        bpf_l4_csum_replace(skb,
                            l4_off + offsetof(struct udphdr, check),
                            old_ip, new_ip,
                            BPF_F_PSEUDO_HDR | BPF_F_MARK_MANGLED_0 | sizeof(__u32));
        bpf_l4_csum_replace(skb,
                            l4_off + offsetof(struct udphdr, check),
                            old_port, new_port,
                            BPF_F_MARK_MANGLED_0 | sizeof(__u16));
        bpf_skb_store_bytes(skb,
                            l4_off + offsetof(struct udphdr, dest),
                            &new_port, sizeof(new_port), 0);
    }

    return TC_ACT_OK;
}
