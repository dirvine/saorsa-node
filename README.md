# saorsa-node

A pure quantum-proof network node for the Saorsa decentralized network.

## Overview

`saorsa-node` is a thin wrapper around `saorsa-core` that provides:

- **Quantum-proof cryptography** - ML-DSA-65 signatures, ML-KEM-768 key exchange
- **Dual IPv4/IPv6 DHT** - Maximum connectivity with separate close groups
- **Sybil resistance** - Multi-layer subnet enforcement, node age requirements
- **EigenTrust reputation** - Automatic bad node detection and removal
- **Geographic routing** - No datacenter concentration in close groups
- **Auto-upgrade** - ML-DSA signed binary verification with rollback
- **ant-node migration** - Automatic data migration from legacy nodes

## Quick Start

```bash
# Build the node
cargo build --release

# Run with default settings
./target/release/saorsa-node

# Run with specific options
./target/release/saorsa-node \
    --root-dir ~/.saorsa \
    --ip-version dual \
    --auto-upgrade

# Migrate data from existing ant-node
./target/release/saorsa-node \
    --migrate-ant-data ~/.local/share/safe/node
```

## CLI Options

```
saorsa-node [OPTIONS]

Options:
    --root-dir <PATH>           Node data directory [default: ~/.saorsa]
    --port <PORT>               Listening port (0 for auto)
    --ip-version <VERSION>      IP version: ipv4, ipv6, or dual [default: dual]
    --bootstrap <ADDR>          Bootstrap peer addresses (can be repeated)
    --migrate-ant-data <PATH>   Path to ant-node data directory to migrate
    --auto-upgrade              Enable automatic upgrades
    --upgrade-channel <CHANNEL> Release channel: stable, beta [default: stable]
    --log-level <LEVEL>         Log level: trace, debug, info, warn, error
    -h, --help                  Print help
    -V, --version               Print version
```

## Architecture

saorsa-node is intentionally minimal. All core functionality is provided by `saorsa-core`:

| Feature | Provider |
|---------|----------|
| Networking | `NetworkCoordinator` |
| DHT | `TrustWeightedKademlia` |
| Trust | `EigenTrustEngine` |
| Security | `SecurityManager` |
| Storage | `ContentStore` |
| Replication | `ReplicationManager` |

saorsa-node adds only:
- **Auto-upgrade system** - GitHub release monitoring with ML-DSA verification
- **ant-node migration** - Read AES-256-GCM-SIV encrypted legacy data
- **CLI wrapper** - User-friendly command-line interface

## Migration from ant-node

When saorsa-node starts, it can automatically scan for and migrate data from existing ant-node installations:

```bash
# Automatic detection
saorsa-node --migrate-ant-data auto

# Specific path
saorsa-node --migrate-ant-data /path/to/ant-node/data
```

The migration process:
1. Scans for ant-node data directories
2. Decrypts AES-256-GCM-SIV encrypted records
3. Uploads data to the saorsa-network
4. Tracks progress for resume capability

## Security

### Quantum-Proof Cryptography

- **Signatures**: ML-DSA-65 (FIPS 204)
- **Key Exchange**: ML-KEM-768 (FIPS 203)
- **Symmetric**: ChaCha20-Poly1305

### Network Hardening

- **Sybil Resistance**: IPv6 node identity binding, subnet limits
- **Rate Limiting**: 100 req/min per node, 500/min per IP
- **Eclipse Protection**: Diversity scoring, ASN limits
- **Geographic Distribution**: 7 regions, latency-aware routing

### Auto-Upgrade Security

All releases are signed with ML-DSA-65. The signing public key is embedded in the binary and cannot be changed without a manual upgrade.

## Configuration

Configuration can be provided via:
1. Command-line arguments (highest priority)
2. Environment variables (`SAORSA_*`)
3. Configuration file (`~/.saorsa/config.toml`)

Example `config.toml`:

```toml
[node]
root_dir = "~/.saorsa"
port = 0  # Auto-select

[network]
ip_version = "dual"
bootstrap = [
    "/ip4/1.2.3.4/udp/12000/quic-v1",
    "/ip6/2001:db8::1/udp/12000/quic-v1"
]

[upgrade]
enabled = true
channel = "stable"
check_interval_hours = 1

[migration]
auto_detect_ant_data = true
```

## Development

```bash
# Run tests
cargo test

# Run with debug logging
RUST_LOG=debug cargo run

# Check for issues
cargo clippy -- -D warnings

# Format code
cargo fmt
```

## License

This project is dual-licensed under MIT and Apache-2.0.

## Related Projects

- [saorsa-core](https://github.com/dirvine/saorsa-core) - Core networking library
- [saorsa-pqc](https://github.com/dirvine/saorsa-pqc) - Post-quantum cryptography
- [saorsa-client](https://github.com/dirvine/saorsa-client) - Client library (bridge layer)
