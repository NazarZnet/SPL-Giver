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
     - `DATABASE_URL` — The MySQL database URL (e.g., `mysql://<username>:<password>@<host>:<port>/<db_name>`).
     - `CLIENT_URL` — The Solana RPC endpoint (e.g., `http://127.0.1:8899`).
     - `GROUPS_YAML` — Path to groups configuration YAML file (e.g., `../groups.yaml`).
     - `BUYERS_CSV` — Path to buyers CSV file (e.g., `../buyers_list.csv`).

   - (Optional) You can generate the main wallet, mint account, buyers list, superuser and mint tokens using the CLI (for testing:
      ```bash
      cargo run -p spl_giver -- --help
      ```

2. **Database & Migrations**
   - The service uses MySQL for persistent storage.
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
     - `users` — Stores all API users and their permissions.

3. **Initial Distribution**
   - For each buyer, an associated token account is created (or checked).
   - The buyer receives the initial percentage of tokens as defined by the group.

4. **Unlock Schedule**
   - For each group, a scheduler is started, which distributes the next portions of tokens to all group buyers at specified intervals.
   - The scheduler runs as a background task and checks the `schedule` table every minute for pending unlocks.

5. **State Tracking & Recovery**
   - The system tracks how many tokens each buyer has received, their remaining balance, and any errors.
   - If the server fails or restarts, it **fetches previous schedule and transaction history from the database** and resumes processing only the pending unlocks. This ensures no double-sending and robust recovery.

---

## File Formats

### YAML (Groups)
```yaml
- id: 1
  spl_share_percent: 0.1
  spl_price_lamports: 100000
  initial_unlock_percent: 0.25
  unlock_interval_seconds: 2592000
  unlock_percent_per_interval: 0.05
- id: 2
  spl_share_percent: 0.2
  spl_price_lamports: 120000000
  initial_unlock_percent: 0.2
  unlock_interval_seconds: 360
  unlock_percent_per_interval: 0.04
```

### CSV (Buyers)
```csv
wallet,paid_lamports,group_id
7G9...abc,10000000,1
8H2...xyz,5000000,2
```

---
# SPL Token Service API Endpoints

## Authentication

### POST /login
Authenticate user and receive access/refresh cookies.

**Request Body:**
```json
{
  "username": "string",
  "password": "string"
}
```

**Response:**
- **200 OK**: Login successful (sets authentication cookies)
- **401 Unauthorized**: Invalid credentials or user not found
- **500 Internal Server Error**: Database or token generation error

---

## General

### GET /
Welcome endpoint for service health check.

**Response:**
```
Welcome to Spl Token Service!
```

---

## Buyers Management

### GET /buyers
Retrieve all buyers or filter by group.

**Query Parameters:**
- `group_id` (optional): Filter buyers by group ID

**Response:**
- **200 OK**: Array of buyer objects
- **500 Internal Server Error**: Database error

**Examples:**
```
GET /buyers                 # Get all buyers
GET /buyers?group_id=1      # Get buyers from group 1
```

### GET /buyers/{wallet}
Get specific buyer by wallet address.

**Path Parameters:**
- `wallet`: Wallet address string

**Response:**
- **200 OK**: Buyer object
- **404 Not Found**: Buyer not found
- **500 Internal Server Error**: Database error

---

## Groups Management

### GET /groups
Retrieve all groups.

**Response:**
- **200 OK**: Array of group objects
- **500 Internal Server Error**: Database error

### GET /groups/{group_id}
Get specific group by ID.

**Path Parameters:**
- `group_id`: Group ID (integer)

**Response:**
- **200 OK**: Group object
- **404 Not Found**: Group not found
- **500 Internal Server Error**: Database error

---

## Schedule Management

### GET /schedule
Retrieve schedules with optional status filtering.

**Query Parameters:**
- `status` (optional): Filter by status (`pending`, `success`, `failed`)

**Response:**
- **200 OK**: Array of schedule objects
- **400 Bad Request**: Invalid status parameter
- **500 Internal Server Error**: Database error

**Examples:**
```
GET /schedule                    # Get all schedules
GET /schedule?status=pending     # Get pending schedules
GET /schedule?status=failed      # Get failed schedules
```

### POST /schedule/retry
Retry all failed schedules.

**Response:**
- **200 OK**: Retry results with statistics
```json
{
  "retried": [...],           // Successfully retried schedules
  "failed": [...],            // Failed retry attempts
  "message": "Status message"
}
```
- **500 Internal Server Error**: Database error

---

## Transactions

### GET /transactions
Retrieve transactions with optional status filtering.

**Query Parameters:**
- `status` (optional): Filter by status (`success`, `failed`)

**Response:**
- **200 OK**: Array of transaction objects
- **400 Bad Request**: Invalid status parameter
- **500 Internal Server Error**: Database error

**Examples:**
```
GET /transactions                   # Get all transactions
GET /transactions?status=success    # Get successful transactions
GET /transactions?status=failed     # Get failed transactions
```

---

## Error Handling

All endpoints follow consistent error handling:

- **400 Bad Request**: Invalid query parameters
- **401 Unauthorized**: Authentication required or failed
- **404 Not Found**: Resource not found
- **500 Internal Server Error**: Database or server errors

Error responses include descriptive messages to help identify the issue.
---

## Technologies

- Rust, Tokio, Actix-web
- Solana SDK, SPL Token-2022
- Serialization: serde, csv-async, serde_yaml

---

## Getting Started

1. Install dependencies (`cargo build`)
2. Create `groups.yaml` and `buyers_list.csv`(Optional you can use CLI to generate it for test)
   ```bash
      cargo run -p spl_giver -- --help
   ```
3. Create file `.env` with necessary variable
3. Run migrations:
   ```bash
   sqlx migrate run
   ```
4. Start the server:
   ```bash
   cargo run -p spl_giver
   ```



