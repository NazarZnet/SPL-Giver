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
   - Create a `.env` file and add the following variables:
     - `MAIN_WALLET` — The admin wallet private key (base58 string). 
     - `MINT_PUBKEY` — The SPL token mint address.
     - `DATABASE_URL` — The SQLite database URL (e.g., `sqlite://spl_giver_history.sqlite`).
     - `CLIENT_URL` — The Solana RPC endpoint (e.g., `http://127.0.1:8899`).
   - Groups are loaded from a YAML file.
   - Buyers are loaded from a CSV file and assigned to groups.
   - The SPL amount available to each group is calculated.

2. **Database & Migrations**
   - The service uses SQLite for persistent storage.
   - **Before running the server, run the database migrations** to create the necessary tables:
     ```sh
     cargo install sqlx-cli
     sqlx migrate run
     ```
   - The main tables are:
     - `groups` — Stores group configuration.
     - `buyers` — Stores buyer information and their progress.
     - `schedule` — Stores unlock schedule for each buyer (when and how much to unlock).
     - `transactions` — Stores all token transfer attempts (success and failure) for audit/history.

3. **Initial Distribution**
   - For each buyer, an associated token account is created (or checked).
   - The buyer receives the initial percentage of tokens as defined by the group.

4. **Unlock Schedule**
   - For each group, a scheduler is started, which distributes the next portions of tokens to all group buyers at specified intervals.
   - The scheduler runs as a background task and checks the `schedule` table every second for pending unlocks.

5. **State Tracking & Recovery**
   - The system tracks how many tokens each buyer has received, their remaining balance, and any errors.
   - If the server fails or restarts, it **fetches previous schedule and transaction history from the database** and resumes processing only the pending unlocks. This ensures no double-sending and robust recovery.

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

---

## Technologies

- Rust, Tokio, Actix-web
- Solana SDK, SPL Token-2022
- Serialization: serde, csv-async, serde_yaml

---

## Getting Started

1. Install dependencies (`cargo build`)
2. Create `groups.yaml` and `buyers_list.csv`(Optional you can uncomment in main.rs line 39 to generate it for test)) in the project root
3. Run migrations:
   ```bash
   sqlx migrate run
   ```
4. Start the server:
   ```bash
   cargo run
   ```



