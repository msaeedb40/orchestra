# syntax=docker/dockerfile:1
ARG BASE_IMAGE=ubuntu:24.04
FROM ${BASE_IMAGE} AS base

ARG KUBERNETES_VERSION=v1.37.1
ARG CRICTL_VERSION=v1.31.1

ENV DEBIAN_FRONTEND=noninteractive
ENV KUBERNETES_VERSION=${KUBERNETES_VERSION}
ENV CONTAINER_RUNTIME_ENDPOINT=unix:///run/containerd/containerd.sock
ENV IMAGE_SERVICE_ENDPOINT=unix:///run/containerd/containerd.sock

# 1. Install base utilities and dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    apt-transport-https \
    ca-certificates \
    curl \
    gnupg \
    iproute2 \
    iptables \
    ebtables \
    ethtool \
    kmod \
    procps \
    socat \
    conntrack \
    containerd \
    runc \
    systemd \
    systemd-sysv \
    udev \
    bash \
    jq \
    && rm -rf /var/lib/apt/lists/*

# 2. Configure containerd default configuration
RUN mkdir -p /etc/containerd && \
    containerd config default | tee /etc/containerd/config.toml > /dev/null && \
    sed -i 's/SystemdCgroup = false/SystemdCgroup = true/' /etc/containerd/config.toml

# 3. Install Kubernetes binaries (kubeadm, kubelet, kubectl)
RUN ARCH=$(dpkg --print-architecture) && \
    K8S_MINOR=$(echo ${KUBERNETES_VERSION} | cut -d'.' -f1,2) && \
    echo "Installing Kubernetes ${KUBERNETES_VERSION} for ${ARCH}..." && \
    mkdir -p /etc/apt/keyrings && \
    curl -fsSL "https://pkgs.k8s.io/core:/stable:/${K8S_MINOR}/deb/Release.key" | gpg --dearmor -o /etc/apt/keyrings/kubernetes-apt-keyring.gpg && \
    echo "deb [signed-by=/etc/apt/keyrings/kubernetes-apt-keyring.gpg] https://pkgs.k8s.io/core:/stable:/${K8S_MINOR}/deb/ /" | tee /etc/apt/sources.list.d/kubernetes.list && \
    apt-get update && \
    apt-get install -y --no-install-recommends \
    kubelet \
    kubeadm \
    kubectl && \
    apt-mark hold kubelet kubeadm kubectl && \
    rm -rf /var/lib/apt/lists/*

# 4. Install crictl
RUN ARCH=$(dpkg --print-architecture) && \
    curl -fsSL "https://github.com/kubernetes-sigs/cri-tools/releases/download/${CRICTL_VERSION}/crictl-${CRICTL_VERSION}-linux-${ARCH}.tar.gz" | tar -C /usr/local/bin -xz

# 5. Setup directories and entrypoint
COPY entrypoint.sh /usr/local/bin/entrypoint.sh
COPY healthcheck.sh /usr/local/bin/healthcheck.sh
RUN chmod +x /usr/local/bin/entrypoint.sh /usr/local/bin/healthcheck.sh

# OCI Labels
LABEL org.opencontainers.image.title="Orchestra Node ${KUBERNETES_VERSION}" \
      org.opencontainers.image.description="Containerized Kubernetes Node for Orchestra" \
      org.opencontainers.image.authors="Orchestra Contributors" \
      org.opencontainers.image.source="https://github.com/msaeedb40/orchestra" \
      io.orchestra.kubernetes.version="${KUBERNETES_VERSION}"

EXPOSE 6443

HEALTHCHECK --interval=10s --timeout=5s --retries=3 CMD ["/usr/local/bin/healthcheck.sh"]

ENTRYPOINT ["/usr/local/bin/entrypoint.sh"]

