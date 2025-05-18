# SPL Giver

**SPL Giver** is a backend service for automated distribution of SPL tokens on Solana, supporting flexible group-based allocation and unlock schedules.

---

## Overview

- The admin wallet is stored on the backend and holds the required amount of SPL tokens (e.g., 10,000,000).
- Buyers are divided into groups; each group has its own share of tokens, SPL price, and unlock schedule.
- Buyer data (wallet address, amount paid in SOL, group) is stored in a CSV file.
- Token distribution happens in stages: an initial portion is unlocked immediately, the rest is unlocked according to the group schedule.

---

## How It Works

1. **Initialization**
   - (Optional) Create `.env` file and add `MAIN_WALLET` and `MINT_PUBKEY`. If these keys not provided server will create new one.
   - Groups are loaded from a YAML file.
   - Buyers are loaded from a CSV file and assigned to groups.
   - The SPL amount available to each group is calculated.

2. **Initial Distribution**
   - For each buyer, an associated token account is created (or checked).
   - The buyer receives the initial percentage of tokens as defined by the group.

3. **Unlock Schedule**
   - For each group, a separate scheduler is started, which distributes the next portions of tokens to all group buyers at specified intervals.

4. **State Tracking**
   - The system tracks how many tokens each buyer has received, their remaining balance, and any errors.

---

## File Formats

### YAML (Groups)
```yaml
- id: 1
  spl_share_percent: 0.1
  spl_total: 1000000
  spl_price: 10.0
  initial_unlock_percent: 0.25
  unlock_interval_seconds: 2592000
  unlock_percent_per_interval: 0.05
- id: 2
  spl_share_percent: 0.2
  spl_total: 2000000
  spl_price: 12.0
  initial_unlock_percent: 0.2
  unlock_interval_seconds: 2592000
  unlock_percent_per_interval: 0.04
```

### CSV (Buyers)
```csv
wallet,paid_sol,group_id
7G9...abc,1000,1
8H2...xyz,500,2
```

---

## Main Endpoints

- `GET /` — Service health check
- `POST /distribute` — Starts token distribution (initial + unlock schedulers)

---

## Technologies

- Rust, Tokio, Actix-web
- Solana SDK, SPL Token-2022
- Serialization: serde, csv-async, serde_yaml

---

## Getting Started

1. Install dependencies (`cargo build`)
2. Create `groups.yaml` and `buyers_list.csv`(Optional you can uncomment in main.rs line 25 to generate it for test)) in the project root
3. Start the server:
   ```sh
   cargo run
   ```
4. Use `POST /distribute` to start token distribution

---

## TODO / Roadmap

- [x] Assign buyers to groups
- [x] Initial token distribution
- [x] Group unlock schedulers
- [ ] API for buyer status
- [ ] Store transaction and error history in a database


