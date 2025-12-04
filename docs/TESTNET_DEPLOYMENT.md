# Testnet Deployment Guide

This guide covers deploying saorsa-node for network testing, including considerations for cloud providers and anti-Sybil protection.

## Overview

The Saorsa network uses multiple anti-Sybil mechanisms that can affect testnet deployments:

1. **IP Diversity Enforcement**: Limits nodes per subnet and ASN
2. **Node Age Verification**: Requires time-based trust accumulation
3. **Geographic Diversity**: Prefers globally distributed nodes

For testing, these protections may need to be relaxed depending on your deployment environment.

## Testnet Configuration

### Using saorsa-core's Testnet Mode

saorsa-core v0.7.0+ provides built-in testnet configurations:

```rust
use saorsa_core::{IPDiversityConfig, NodeAgeConfig};

// Relaxed IP diversity for cloud testing (e.g., Digital Ocean)
let diversity_config = IPDiversityConfig::testnet();
// - Allows up to 5000 nodes from the same ASN
// - Allows 100 nodes per /64 subnet
// - Disables geographic diversity requirements

// Relaxed node age requirements for fast testing
let age_config = NodeAgeConfig::testnet();
// - No minimum age for replication
// - No minimum age for critical operations
// - Instant veteran status

// For local development, use permissive mode:
let diversity_config = IPDiversityConfig::permissive();
let age_config = NodeAgeConfig::permissive();
```

### saorsa-node Configuration

In your `saorsa-node` configuration file:

```toml
# config.toml for testnet deployment

[network]
# Use testnet-specific bootstrap nodes
bootstrap = ["<bootstrap-node-address>"]

# Enable testnet mode
testnet_mode = true

[security]
# Relax diversity requirements for single-provider deployments
max_nodes_per_asn = 5000
max_nodes_per_subnet_64 = 100
enforce_age_requirements = false
```

## Cloud Provider Considerations

### Digital Ocean

**ASN Information**: All Digital Ocean nodes share **ASN 14061**.

**Challenge**: Default anti-Sybil protection limits nodes to 20 per ASN, which would prevent deploying more than 20 test nodes.

**Solution**: Use `IPDiversityConfig::testnet()` which allows up to 5000 nodes from the same ASN.

```bash
# Example deployment script for Digital Ocean
export SAORSA_TESTNET_MODE=true
export SAORSA_MAX_NODES_PER_ASN=5000

./saorsa-node --config testnet.toml
```

### AWS

**ASN Information**: AWS uses multiple ASNs:
- AS16509 (primary)
- AS14618 (secondary)

**Advantage**: More natural diversity across availability zones.

**Recommendation**: Deploy across multiple availability zones for geographic diversity even in testnet mode.

### Google Cloud Platform

**ASN Information**:
- AS15169 (primary)
- AS396982 (Cloud services)

**Recommendation**: Similar to AWS, leverage multiple regions for better testing coverage.

## Multi-Cloud Deployment Strategy

For realistic network testing, we recommend a multi-cloud deployment:

### Tier 1: Development Testing (Single Provider)
- 5-10 nodes on Digital Ocean
- Use `testnet()` configuration
- Suitable for basic functionality testing

```bash
# Deploy 5 nodes on DO with testnet config
for i in $(seq 1 5); do
    doctl compute droplet create \
        saorsa-node-$i \
        --image saorsa-node:latest \
        --size s-1vcpu-2gb \
        --region nyc1
done
```

### Tier 2: Integration Testing (Multi-Region, Single Provider)
- 20-50 nodes across multiple regions
- Tests geographic routing and latency handling
- Still uses testnet configuration for ASN diversity

### Tier 3: Pre-Production Testing (Multi-Cloud)
- 50+ nodes across multiple providers
- Uses default (production) diversity settings
- Tests real anti-Sybil behavior

Recommended distribution:
- 40% Digital Ocean (various regions)
- 30% AWS (multiple AZs)
- 20% Hetzner (EU presence)
- 10% OVH/Scaleway (additional diversity)

```bash
# Example multi-cloud deployment
# DO nodes (40%)
terraform apply -target=module.digitalocean

# AWS nodes (30%)
terraform apply -target=module.aws

# Hetzner nodes (20%)
terraform apply -target=module.hetzner

# OVH nodes (10%)
terraform apply -target=module.ovh
```

## Configuration Reference

### IPDiversityConfig Options

| Field | Default | Testnet | Description |
|-------|---------|---------|-------------|
| max_nodes_per_64 | 1 | 100 | Max nodes per /64 subnet |
| max_nodes_per_48 | 3 | 500 | Max nodes per /48 allocation |
| max_nodes_per_32 | 10 | 1000 | Max nodes per /32 region |
| max_nodes_per_asn | 20 | 5000 | Max nodes per ASN |
| enable_geolocation_check | true | false | Enable geo checks |
| min_geographic_diversity | 3 | 1 | Min countries required |

### NodeAgeConfig Options

| Field | Default | Testnet | Description |
|-------|---------|---------|-------------|
| min_replication_age_secs | 3600 | 0 | Min age for replication |
| min_critical_ops_age_secs | 86400 | 0 | Min age for critical ops |
| enforce_age_requirements | true | false | Enforce age checks |
| veteran_age_secs | 604800 | 0 | Age for veteran status |

## Monitoring Testnet Health

### Key Metrics to Watch

1. **Network Formation Time**: How long until all nodes discover each other
2. **DHT Convergence**: Time for routing tables to stabilize
3. **Replication Success Rate**: Percentage of successful data replications
4. **Cross-ASN Latency**: Network performance across providers

### Example Monitoring Commands

```bash
# Check node status
curl http://node-ip:8080/health

# Check connected peers
curl http://node-ip:8080/peers | jq '.peer_count'

# Check DHT routing table
curl http://node-ip:8080/dht/stats
```

## Security Considerations

### Production vs. Testnet

| Aspect | Production | Testnet |
|--------|------------|---------|
| ASN Limits | 20 nodes/ASN | 5000 nodes/ASN |
| Age Requirements | Enforced | Disabled |
| Geographic Diversity | Required | Optional |
| Sybil Protection | Full | Minimal |

### Never Use Testnet Config in Production

The testnet configuration significantly weakens Sybil attack protection. Always ensure production deployments use the default configuration:

```rust
// Production (default - safe)
let diversity_config = IPDiversityConfig::default();
let age_config = NodeAgeConfig::default();

// Check if accidentally using testnet config
if diversity_config.is_relaxed() || age_config.is_relaxed() {
    panic!("DANGER: Testnet configuration detected in production!");
}
```

## Troubleshooting

### Nodes Can't Join Network

**Symptom**: "IP diversity limits exceeded" error

**Cause**: Too many nodes from the same ASN/subnet

**Solution**:
```rust
// In your node configuration
let config = IPDiversityConfig::testnet();
```

### Nodes Can't Replicate Data

**Symptom**: "Node age too low for replication" error

**Cause**: Node hasn't met age requirements

**Solution**:
```rust
// In your node configuration
let config = NodeAgeConfig::testnet();
```

### Slow Network Formation

**Symptom**: Nodes taking too long to discover peers

**Cause**: Bootstrap nodes may be overloaded

**Solution**: Add multiple bootstrap nodes across regions:
```toml
bootstrap = [
    "bootstrap1.testnet.saorsa.io:9000",
    "bootstrap2.testnet.saorsa.io:9000",
    "bootstrap3.testnet.saorsa.io:9000"
]
```

## Example: Complete Digital Ocean Testnet

```bash
#!/bin/bash
# deploy-do-testnet.sh

# Configuration
NUM_NODES=10
REGION="nyc1"
SIZE="s-2vcpu-4gb"

# Create bootstrap node first
echo "Creating bootstrap node..."
BOOTSTRAP_IP=$(doctl compute droplet create \
    saorsa-bootstrap \
    --image saorsa-node:latest \
    --size $SIZE \
    --region $REGION \
    --user-data "SAORSA_BOOTSTRAP=true" \
    --wait \
    --format PublicIPv4 --no-header)

echo "Bootstrap node: $BOOTSTRAP_IP"

# Create regular nodes
for i in $(seq 1 $NUM_NODES); do
    echo "Creating node $i..."
    doctl compute droplet create \
        saorsa-node-$i \
        --image saorsa-node:latest \
        --size $SIZE \
        --region $REGION \
        --user-data "SAORSA_BOOTSTRAP_PEERS=$BOOTSTRAP_IP:9000" \
        --wait
done

echo "Testnet deployed with $NUM_NODES nodes"
```

## Next Steps

1. Deploy your testnet using this guide
2. Run integration tests against the testnet
3. Monitor for issues and collect metrics
4. Gradually transition to multi-cloud for pre-production
5. Final production deployment with default (strict) configuration
