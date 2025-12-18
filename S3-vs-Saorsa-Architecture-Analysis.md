# Comprehensive Architecture Analysis: Saorsa + Communitas vs Amazon S3

**Date**: December 18, 2025
**Author**: Architecture Research Report
**Version**: 1.0

---

## Executive Summary

This report provides a comprehensive comparison between Amazon S3's centralized object storage model and the Saorsa Network + Communitas decentralized architecture. The key differentiator is the **pay-once-store-forever archival model** with upfront token payments, versus S3's **recurring subscription-based** approach.

**Key Findings**:
- Saorsa excels for permanent archives with superior economics over 10+ year horizons
- Communitas provides Byzantine-resilient metadata management via CRDTs
- The architecture survives catastrophic infrastructure failures where S3 would fail
- Post-quantum cryptography throughout provides future-proof security
- Gaps exist in public data naming/discovery and solo user recovery

---

## Table of Contents

1. [The Complete Storage Ecosystem](#1-the-complete-storage-ecosystem)
2. [Fundamental Architecture Comparison](#2-fundamental-architecture-comparison)
3. [The Metadata Problem](#3-the-metadata-problem-deeply-analyzed)
4. [Expert Analysis - Strengths](#4-expert-analysis---strengths)
5. [Expert Analysis - Challenges & Gaps](#5-expert-analysis---challenges--gaps)
6. [Strategic Comparison Matrix](#6-strategic-comparison-matrix)
7. [The Grand Vision](#7-the-grand-vision)
8. [Final Recommendations](#8-final-expert-recommendations)
9. [Sources](#9-sources)

---

## 1. The Complete Storage Ecosystem

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                        USER DATA LIFECYCLE                                   │
└─────────────────────────────────────────────────────────────────────────────┘
                                    │
                    ┌───────────────┴───────────────┐
                    ▼                               ▼
           ┌───────────────┐               ┌───────────────┐
           │  PUBLIC DATA  │               │ PRIVATE DATA  │
           └───────┬───────┘               └───────┬───────┘
                   │                               │
                   ▼                               ▼
    ┌──────────────────────────┐    ┌──────────────────────────────────┐
    │    self_encryption       │    │       self_encryption            │
    │  ┌────────────────────┐  │    │  ┌────────────────────────────┐  │
    │  │ File → Chunks      │  │    │  │ File → Chunks              │  │
    │  │ Chunks → DataMap   │  │    │  │ Chunks → DataMap           │  │
    │  └────────────────────┘  │    │  └────────────────────────────┘  │
    └───────────┬──────────────┘    └────────────────┬─────────────────┘
                │                                    │
                ▼                                    ▼
    ┌──────────────────────────┐    ┌──────────────────────────────────┐
    │      SAORSA NETWORK      │    │         COMMUNITAS               │
    │  ┌────────────────────┐  │    │  ┌────────────────────────────┐  │
    │  │ Chunks: DHT        │  │    │  │ Chunks: Saorsa DHT         │  │
    │  │ DataMap: as Chunk  │◄─┼────┼──┤ DataMap: CRDT + MLS        │  │
    │  │ (content-addressed)│  │    │  │ (shared, encrypted)        │  │
    │  └────────────────────┘  │    │  └────────────────────────────┘  │
    │                          │    │                                  │
    │  Payment: One-time token │    │  Sync: Plumtree gossip          │
    │  Duration: Forever       │    │  Encryption: ChaCha20-Poly1305  │
    │  Retrieval: Free         │    │  Groups: MLS per-topic          │
    └──────────────────────────┘    └──────────────────────────────────┘
```

### Self-Encryption Process

The `self_encryption` library (maidsafe/self_encryption) implements convergent encryption:

1. **Compression**: Brotli compression
2. **Encryption**: AES-256-CBC with keys derived from neighboring chunk hashes
3. **Obfuscation**: XOR for additional security

**Key Properties**:
- Chunk size: Up to 1MB per chunk
- Keys: SHA3-256 derived from combined hashes of 3 consecutive chunks
- DataMap: Contains pre/post encryption hashes for retrieval and verification
- Hierarchical: Large files create nested DataMaps automatically

---

## 2. Fundamental Architecture Comparison

### Amazon S3 Mental Model

```
┌─────────────────────────────────────────────────────────────────┐
│                     AMAZON S3 PARADIGM                          │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Namespace: s3://bucket-name/path/to/object.ext                │
│             └───────┬───────┘└────────┬───────┘                │
│              Location       Path (mutable pointer)              │
│                                                                 │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │ OBJECT = { key, body, metadata, ACL, versioning }       │   │
│  │                                                         │   │
│  │ • Key is arbitrary string (user-chosen)                 │   │
│  │ • Body can be overwritten (mutable)                     │   │
│  │ • Metadata is key-value pairs (user-defined)            │   │
│  │ • Versioning is optional (costs extra)                  │   │
│  └─────────────────────────────────────────────────────────┘   │
│                                                                 │
│  Trust Model: You trust AWS completely                         │
│  - AWS controls access                                          │
│  - AWS can read your data (unless client-side encrypted)       │
│  - AWS can delete your data                                     │
│  - AWS can raise prices                                         │
│  - AWS can terminate your account                               │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### Saorsa + Communitas Mental Model

```
┌─────────────────────────────────────────────────────────────────┐
│                SAORSA + COMMUNITAS PARADIGM                     │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Namespace: XorName (32-byte SHA256 hash of content)           │
│             └─────────────────┬─────────────────┘              │
│                    Content IS the address                       │
│                                                                 │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │ CHUNK = { address: SHA256(content), content: Bytes }    │   │
│  │                                                         │   │
│  │ • Address is derived (not chosen)                       │   │
│  │ • Content is immutable (by cryptographic design)        │   │
│  │ • No metadata in storage layer (by design)              │   │
│  │ • Deduplication is automatic                            │   │
│  └─────────────────────────────────────────────────────────┘   │
│                                                                 │
│  Trust Model: Trust mathematics, not corporations              │
│  - No single entity controls access                             │
│  - Data is self-verifying (address = hash)                     │
│  - No one can tamper without detection                         │
│  - No one can selectively censor                                │
│  - Price paid once, storage guaranteed by economics            │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### Core Architecture Differences

| Aspect | Amazon S3 | Saorsa Network |
|--------|-----------|----------------|
| **Addressing** | Location-based: `s3://bucket/key` | Content-based: `XorName = SHA256(content)` |
| **Mutability** | Mutable objects (can overwrite) | **Immutable** - content defines address |
| **Replication** | 3+ Availability Zones per region | DHT with configurable replica count (default 4) |
| **Durability** | 99.999999999% (11 nines) | Emergent from network redundancy |
| **Max Object Size** | 5TB (multipart upload) | 4MB per chunk (1MB for self-encrypted) |
| **Consistency** | Strong read-after-write | Eventual consistency (DHT) |
| **Metadata** | Key-value tags attached to objects | Separate network (Communitas) |

---

## 3. The Metadata Problem (Deeply Analyzed)

### The Core Challenge

The architecture creates a **separation of concerns**:

| Layer | What It Stores | Addressing | Mutability |
|-------|---------------|------------|------------|
| **Saorsa (chunks)** | Raw encrypted data | Content-addressed | Immutable |
| **Saorsa (DataMaps)** | Chunk references for public data | Content-addressed | Immutable |
| **Communitas (CRDTs)** | DataMaps + metadata for private data | Identity-based | Mutable (CRDT) |

### Dual-Track Metadata Solution

#### Public Data Path

```
File "research-paper.pdf"
     │
     ▼ self_encrypt()
┌─────────────────────────────────────────────────────────────────────┐
│  Encrypted Chunks (stored in Saorsa DHT)                            │
│  c1: 0xabc123... (1MB)                                              │
│  c2: 0xdef456... (1MB)                                              │
│  c3: 0x789ghi... (500KB)                                            │
└─────────────────────────────────────────────────────────────────────┘
     │
     ▼ DataMap serialized as chunk
┌─────────────────────────────────────────────────────────────────────┐
│  DataMap Chunk: 0xDATAMAP_HASH...                                   │
│  Content: { chunks: [c1, c2, c3], src_hashes: [...] }               │
│  PUBLISHED PUBLICLY - Anyone with hash can retrieve                 │
└─────────────────────────────────────────────────────────────────────┘

DISCOVERY: External publication (website, DNS, social media, QR code)
```

#### Private Data Path

```
File "family-photos.zip"
     │
     ▼ self_encrypt()
┌─────────────────────────────────────────────────────────────────────┐
│  Encrypted Chunks (stored in Saorsa DHT)                            │
│  Same as public path - chunks don't know if they're "private"       │
└─────────────────────────────────────────────────────────────────────┘
     │
     ▼ DataMap stored in Communitas
┌─────────────────────────────────────────────────────────────────────┐
│  Communitas CRDT Document (yrs Y.Map)                               │
│  ┌───────────────────────────────────────────────────────────────┐  │
│  │ MemberDocument {                                              │  │
│  │   files: OR-Set<FileEntry> {                                  │  │
│  │     "family-photos.zip" → {                                   │  │
│  │       data_map: DataMap { chunks: [...] },                    │  │
│  │       metadata: { size, created, permissions }                │  │
│  │     }                                                         │  │
│  │   }                                                           │  │
│  │ }                                                             │  │
│  └───────────────────────────────────────────────────────────────┘  │
│                                                                      │
│  Encryption: ChaCha20-Poly1305 (per-group key via MLS)              │
│  Sync: Plumtree gossip to group members                             │
│  Storage: Each member's local vault                                  │
└─────────────────────────────────────────────────────────────────────┘

DISCOVERY: Group members have DataMap in their synced CRDT state
ACCESS CONTROL: MLS group membership = who can decrypt DataMaps
```

---

## 4. Expert Analysis - Strengths

### 4.1 Cryptographic Separation of Concerns

| Property | S3 | Saorsa+Communitas |
|----------|----|--------------------|
| **Data at rest encryption** | Optional, AWS-managed keys | Mandatory, user-controlled |
| **Access control** | IAM policies (revocable) | Cryptographic (mathematical) |
| **Metadata privacy** | AWS can read all metadata | Metadata encrypted in Communitas |
| **Deduplication** | None (location-addressed) | Automatic (content-addressed) |

**Expert Opinion**: This is the correct architecture. Data storage should be "dumb" (just bytes), and intelligence (access control, organization, search) should live in a separate layer. S3 conflates these concerns.

### 4.2 Byzantine-Resilient Metadata Sync

```
Communitas CRDT Properties:
├── OR-Set for file lists → Add/remove always converges
├── LWW-Register for metadata → Last write wins deterministically
├── Vector Clocks → Causal ordering preserved
├── Anti-Entropy → Eventually consistent (provably)
└── MLS Groups → Forward secrecy + post-compromise security
```

**Expert Opinion**: Using CRDTs for metadata is superior to S3's strong consistency for collaborative scenarios:
- Partition-tolerant (works during network splits)
- Available (always writable locally)
- Convergent (mathematically guaranteed)

### 4.3 Catastrophic Failure Resilience

| Scenario | S3 | Communitas |
|----------|-----|------------|
| Global internet fragments | Complete failure | Local vault has all synced data |
| AWS outage | Data inaccessible | Bluetooth/local network for peer discovery |
| Regional disaster | Cross-region replication (extra cost) | CRDT sync resumes when any peers reconnect |
| Account termination | Data lost | Data survives on each participant's device |

**Expert Opinion**: This is the killer feature. In a world of increasing geopolitical instability, internet shutdowns, and infrastructure attacks, this architecture survives scenarios where S3 becomes a single point of failure.

### 4.4 Economic Model Superiority for Archives

#### 10-Year TCO Comparison (1TB archival data)

**S3 Glacier Deep Archive:**
| Cost Component | Calculation | Total |
|----------------|-------------|-------|
| Storage | $0.00099/GB/month × 1000GB × 120 months | $118.80 |
| PUT requests | ~$0.05/1000 × initial upload | ~$0.50 |
| Retrieval (once/year) | $0.02/GB × 1000GB × 10 retrievals | $200.00 |
| Data transfer out | $0.09/GB × 1000GB × 10 transfers | $900.00 |
| **TOTAL** | | **~$1,220+** |

**Saorsa:**
| Cost Component | Calculation | Total |
|----------------|-------------|-------|
| One-time token payment | $X (set by network economics) | $X |
| Retrieval | Free | $0 |
| Egress | Free | $0 |
| **TOTAL** | | **$X (fixed, forever)** |

**Break-even**: If $X < $1,220, Saorsa wins for 10+ year archives.

---

## 5. Expert Analysis - Challenges & Gaps

### 5.1 The "Publishable Address" Problem

**Current State**: Public DataMaps are stored as chunks with content-addressed XorNames. But how do users *find* them?

```
Problem:
User A uploads "important-document.pdf"
  → DataMap stored at XorName: 0x7f3a9b2c...

User B wants to retrieve it
  → How does User B know the XorName?
```

**Current Options:**
- Out-of-band sharing (email, QR code, social media)
- DNS TXT record pointing to XorName
- Blockchain registration (expensive, overkill)
- External directory service (defeats decentralization)

**Recommended Solution: Petnames in Communitas**

```
User's PrivateArchive (in Communitas CRDT):
┌──────────────────────────────────────────────┐
│ my_published_files: OR-Set<(name, XorName)>  │
│   "research-paper" → 0x7f3a9b2c...           │
│   "family-archive" → 0x9d2e4f1a...           │
└──────────────────────────────────────────────┘

Share via: four-words identity + file petname
"Get 'research-paper' from david-lion-castle-wind"
```

### 5.2 Immutable Public Collections

**Challenge**: PublicArchives are immutable. Adding a file means creating a new archive with a new XorName.

```
Scenario: User maintains a public photo gallery

Day 1: Upload 100 photos → Archive at 0xABC...
Day 2: Add 10 photos → NEW Archive at 0xDEF...
Day 3: Add 5 photos → NEW Archive at 0xGHI...

Problem: All links to 0xABC... are now stale
```

**Recommended Solution: Mutable Pointers via Communitas**

```
Communitas CRDT (user's public profile):
┌───────────────────────────────────────────────────────────────┐
│ public_sites: LWW-Register<SitePointer> {                     │
│   "photo-gallery" → {                                         │
│     current_archive: XorName(0xGHI...),  // mutable!          │
│     previous_versions: [0xABC..., 0xDEF...],                  │
│     last_updated: timestamp                                   │
│   }                                                           │
│ }                                                             │
└───────────────────────────────────────────────────────────────┘

Resolution Flow:
1. Query "david's photo-gallery" via FOAF gossip
2. David's peers return LWW-Register value
3. Fetch archive from Saorsa at current XorName
```

### 5.3 Recovery Scenarios

| Scenario | S3 | Saorsa+Communitas |
|----------|-----|-------------------|
| User loses all devices | Login to AWS console, data intact | **PROBLEM**: DataMaps lost with devices |

**Current Mitigation:**
- Favourite contacts store encrypted replicas
- FEC (Reed-Solomon) provides redundancy
- Group members have full CRDT state

**Gap**: Solo users with no groups have no recovery path for private DataMaps.

**Recommended Solution: Social Recovery via Shamir Secret Sharing**

```
Shamir Secret Sharing for Master Key:
┌───────────────────────────────────────────────────────────────┐
│ vault_master_key → split into N shares (threshold K)          │
│                                                               │
│ Share 1 → Trusted Contact A (encrypted to their ML-KEM)      │
│ Share 2 → Trusted Contact B                                   │
│ Share 3 → Trusted Contact C                                   │
│ Share 4 → User's backup (paper, safety deposit)               │
│ Share 5 → Optional: Escrow service                            │
│                                                               │
│ Recovery: Collect K shares → reconstruct master key           │
│ → Decrypt vault → Access all DataMaps                         │
└───────────────────────────────────────────────────────────────┘
```

---

## 6. Strategic Comparison Matrix

### 6.1 Feature-by-Feature Analysis

| Dimension | Amazon S3 | Saorsa + Communitas | Winner |
|-----------|-----------|---------------------|--------|
| **Addressing** | Location-based (mutable) | Content-based (immutable) | *Depends on use case* |
| **Deduplication** | None | Automatic | **Saorsa** |
| **Consistency** | Strong | Eventual (CRDT) | *S3 for transactions* |
| **Availability** | 99.99% | Depends on network | **S3** (for now) |
| **Durability** | 11 nines | Emergent from replication | **S3** (proven) |
| **Censorship Resistance** | None | Strong | **Saorsa** |
| **Privacy** | AWS can read | End-to-end encrypted | **Saorsa+Communitas** |
| **Cost Model** | Recurring | One-time | **Saorsa** (for archives) |
| **Metadata Flexibility** | Rich (tags, ACLs) | Separate layer (Communitas) | *S3 more convenient* |
| **Ecosystem** | Massive | Emerging | **S3** |
| **Offline Support** | None | Full (Communitas) | **Communitas** |
| **Disaster Recovery** | AWS manages | User-controlled + social | *Depends on scenario* |
| **Quantum Safety** | Not yet | ML-DSA, ML-KEM, ChaCha20 | **Saorsa+Communitas** |

### 6.2 Use Case Fit Analysis

| Use Case | Best Choice | Reasoning |
|----------|-------------|-----------|
| **Enterprise file storage** | S3 | IAM, compliance, SLAs |
| **Permanent archives** | Saorsa | Pay once, immutable proof |
| **Private collaboration** | Communitas | E2E encrypted, offline-first |
| **Public websites** | S3 (for now) | CDN, HTTPS, DNS integration |
| **Research data** | Saorsa | Content-addressed citations |
| **Legal/compliance** | Saorsa | Immutability, audit trail |
| **Real-time apps** | S3 | Strong consistency |
| **Censorship-resistant publishing** | Saorsa + Communitas | No single point of control |
| **Family/friend file sharing** | Communitas | Private groups, local sync |
| **Disaster-resilient backup** | Communitas | Survives infrastructure failures |

### 6.3 S3 Pricing Reference (US East, 2025)

| Storage Class | $/GB/Month | Retrieval Cost | Retrieval Time |
|---------------|------------|----------------|----------------|
| S3 Standard | $0.023 | Free | Immediate |
| S3 Glacier Instant | $0.004 | $0.03/GB | Milliseconds |
| S3 Glacier Flexible | $0.0036 | $0.01-$0.03/GB | Minutes to hours |
| S3 Glacier Deep Archive | $0.00099 | $0.02/GB | 12-48 hours |

---

## 7. The Grand Vision

### 7.1 The Decentralized Data Stack

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                                                                             │
│                    THE DECENTRALIZED DATA STACK                            │
│                                                                             │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  Layer 4: Applications                                                      │
│  ┌───────────────────────────────────────────────────────────────────────┐ │
│  │  Communitas UI │ Future Apps │ Third-Party Integrations              │ │
│  └───────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
│  Layer 3: Collaboration (Communitas)                                        │
│  ┌───────────────────────────────────────────────────────────────────────┐ │
│  │  CRDTs │ MLS Groups │ Presence │ Private DataMaps │ Access Control   │ │
│  └───────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
│  Layer 2: Communication (saorsa-gossip)                                     │
│  ┌───────────────────────────────────────────────────────────────────────┐ │
│  │  HyParView │ Plumtree │ SWIM │ FOAF │ Anti-Entropy │ NAT Traversal   │ │
│  └───────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
│  Layer 1: Storage (Saorsa Network)                                          │
│  ┌───────────────────────────────────────────────────────────────────────┐ │
│  │  Chunks │ DHT │ Content Addressing │ Payment │ Self-Encryption       │ │
│  └───────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
│  Layer 0: Transport (ant-quic)                                              │
│  ┌───────────────────────────────────────────────────────────────────────┐ │
│  │  QUIC │ ML-DSA Signatures │ ML-KEM Key Exchange │ Multi-Transport    │ │
│  └───────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 7.2 Stack Comparison

**Amazon S3 Stack:**
```
├── Application: Your code
├── SDK: AWS SDK (proprietary)
├── API: S3 REST API (HTTP)
├── Storage: S3 (proprietary, closed)
├── Network: AWS Global Infrastructure (proprietary)
└── Trust: Amazon (single point of failure/control)
```

**Saorsa + Communitas Stack:**
```
├── Application: Communitas (open source)
├── SDK: Rust crates (open source)
├── Protocol: Content-addressed chunks (open standard)
├── Storage: DHT (decentralized, open)
├── Network: saorsa-gossip (open, multi-transport)
└── Trust: Mathematics (cryptographic guarantees)
```

### 7.3 Communitas Technical Details

**CRDT Implementation (yrs v0.19):**
- **LWW (Last-Write-Wins)**: Scalar values with Lamport timestamps
- **G-Counter**: Grow-only counters per peer
- **OR-Set**: Concurrent add/remove with unique operation IDs
- **Tombstone**: Soft-delete markers with LWW semantics

**Encryption Stack:**
- **ML-DSA (FIPS 204)**: Post-quantum signatures
- **ML-KEM (FIPS 203)**: Post-quantum key encapsulation
- **ChaCha20-Poly1305**: Symmetric authenticated encryption
- **PBKDF2**: Key derivation (100,000 iterations)

**Gossip Protocol (saorsa-gossip):**
- **HyParView**: Peer membership (8 active, 64+ passive)
- **SWIM**: Failure detection (3-second suspect timeout)
- **Plumtree**: Message dissemination (eager push + lazy gossip)
- **Anti-Entropy**: CRDT reconciliation

**Storage Architecture:**
```
vault_dir/{four_words}/
├── metadata.json              # Vault metadata
├── files/                     # Secret group files (encrypted)
├── group_shards/              # Reed-Solomon shards from groups
├── dht_cache/                 # Local DHT cache
├── metadata/                  # Storage indices
├── temp/                      # Temporary files
└── web/                       # Public markdown content
```

---

## 8. Final Expert Recommendations

### 8.1 Launch Strategy (Confirmed Correct)

Your focus is correct:
- **Communitas + Saorsa as hard archive first**
- Private shared data working
- Public immutable data working
- Defer public mutable data (websites, etc.)

### 8.2 Address the Naming Gap (Priority: High)

Before public launch, solve "how do I share this file?":
- Implement petnames in Communitas CRDT
- Enable "get X from four-words" pattern
- Consider QR code + deep link generation

### 8.3 Document the Trust Model (Priority: High)

Create clear documentation explaining:
- What survives if Saorsa Labs disappears
- What survives if all your devices are lost
- What survives if the internet fragments
- Recovery paths for each scenario

### 8.4 Consider S3-Compatible Gateway (Priority: Future)

For adoption, an S3-compatible API would unlock:
- Existing backup tools (rclone, restic)
- Enterprise integration paths
- Migration from existing S3 workloads

### 8.5 Quantify the Economics (Priority: Medium)

Publish clear pricing comparisons:
- "Store 1TB for 10 years: S3 = $X, Saorsa = $Y"
- Break-even calculator
- Total cost of ownership models

### 8.6 Summary Assessment

| Aspect | Assessment |
|--------|------------|
| **Storage Layer (Saorsa)** | Excellent - content-addressed, immutable, paid-once |
| **Metadata Layer (Communitas)** | Excellent - CRDT-based, encrypted, resilient |
| **Gossip Layer** | Production-ready - HyParView+Plumtree+SWIM |
| **Cryptography** | Future-proof - post-quantum throughout |
| **Public Data Naming** | Gap - needs petnames/registry |
| **Solo User Recovery** | Gap - needs social recovery |
| **S3 Ecosystem Compatibility** | Future consideration |

---

## 9. Sources

### Amazon S3 Documentation
- [Amazon S3 API Reference](https://docs.aws.amazon.com/AmazonS3/latest/API/Welcome.html)
- [Amazon S3 Strong Consistency](https://aws.amazon.com/s3/consistency/)
- [S3 Glacier Storage Classes](https://aws.amazon.com/s3/storage-classes/glacier/)
- [AWS S3 Pricing](https://aws.amazon.com/s3/pricing/)
- [All Things Distributed: S3 Consistency Deep Dive](https://www.allthingsdistributed.com/2021/04/s3-strong-consistency.html)

### Decentralized Storage Comparisons
- [Amazon S3 vs IPFS Comparison](https://stackshare.io/stackups/amazon-s3-vs-ipfs)
- [Filebase: S3-Compatible IPFS](https://filebase.com/blog/introducing-support-for-ipfs-backed-by-decentralized-storage/)
- [Top Decentralized Storage Platforms 2025](https://blog.apillon.io/the-top-7-decentralized-cloud-storage-platforms-in-2023-d9bdfc0e1f2d/)

### MaidSafe/Saorsa Technical References
- [maidsafe/self_encryption](https://github.com/maidsafe/self_encryption)
- saorsa-node source code analysis
- communitas source code analysis
- saorsa-gossip source code analysis

### Pricing Guides
- [nOps AWS S3 Pricing Guide](https://www.nops.io/blog/aws-s3-pricing/)
- [CloudZero S3 Pricing Guide 2025](https://www.cloudzero.com/blog/s3-pricing/)

---

## Conclusion

**Your architecture is fundamentally sound and addresses real limitations of centralized storage.**

The key insight: You're not building "decentralized S3" - you're building something more sophisticated. S3 is a bucket of mutable objects. You're building a **permanent archive with a collaboration layer** - fundamentally different and arguably more valuable for the use cases you're targeting.

The combination of:
- **Immutable, content-addressed storage** (Saorsa)
- **Byzantine-resilient metadata** (Communitas CRDTs)
- **Post-quantum cryptography** throughout
- **Pay-once economics**

Creates a system that is technically superior to S3 for archival and collaborative use cases, while being economically superior for long-term storage horizons.

---

*Report generated December 18, 2025*
