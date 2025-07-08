# DKLs23 Protocol Overview

## Introduction

DKLs23 is a state-of-the-art threshold ECDSA protocol that enables multiple parties to jointly sign messages without ever reconstructing the private key. This document provides an overview of the protocol as implemented in this project.

## Key Features

- **Three-Round Signing**: Matches the round complexity of Schnorr signatures
- **UC Security**: Universally Composable security guarantees
- **No Explicit ZK Proofs**: Light protocol with straightforward analysis
- **Black-Box 2P-MUL**: Uses OT-based multiplication that satisfies UC

## Protocol Phases

### 1. Distributed Key Generation (DKG)

The DKG protocol generates shares of a secret key such that:
- No single party knows the complete private key
- Any t-of-n parties can collaborate to sign
- The public key is known to all parties

**Rounds:**
1. Each party commits to a random polynomial
2. Parties exchange secret shares
3. Parties verify shares against commitments

### 2. Key Refresh

Allows parties to refresh their shares without changing the public key:
- Useful for proactive security
- Limits the window of vulnerability
- Old shares become useless after refresh

### 3. Distributed Signature Generation (DSG)

Generates ECDSA signatures using threshold shares:

**Round 1**: Generate nonce shares
- Each party generates random k_i, γ_i
- Broadcast commitments to these values

**Round 2**: Multiplicative-to-Additive (MtA)
- Parties run pairwise MtA protocols
- Compute shares of k^(-1) and k^(-1) * x

**Round 3**: Partial Signatures
- Each party computes partial signature
- Combine partials to get final signature

## Security Model

The protocol achieves security against:
- **Passive Adversaries**: Honest-but-curious parties
- **Active Adversaries**: Malicious parties (with proper ZK proofs)
- **Threshold Adversaries**: Up to t-1 corrupted parties

## References

- [DKLs23 Paper](https://eprint.iacr.org/2023/765.pdf)
- [SoftSpokenOT](https://eprint.iacr.org/2022/192.pdf)
- [Endemic OT](https://eprint.iacr.org/2019/706.pdf)
