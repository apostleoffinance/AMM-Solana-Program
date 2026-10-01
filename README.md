# amm-video

A Solana constant-product automated market maker (AMM) implemented with Anchor. A pool holds two SPL tokens, issues LP tokens to liquidity providers, supports proportional withdrawals, and swaps between the assets while collecting protocol fees in treasury token accounts.

> This repository is an instructional prototype, not an audited or production-ready exchange.

## Features

- One pool per unique `seed` and ordered pair of token mints.
- SPL Token vaults controlled by a pool configuration PDA.
- LP token mint controlled by the same PDA; LP tokens use six decimals.
- Initial deposits require both pool tokens to use six decimals. The first LP issuance equals the larger of the two initial deposit amounts.
- Later deposits and withdrawals are proportional to current pool reserves and LP-token supply, with caller-specified slippage bounds.
- Swaps use the constant-product formula and enforce a caller-specified minimum output.
- Configurable swap fee in basis points. The fee must be less than 10,000 bps (100%).
- Separate X and Y treasury associated token accounts, owned by an immutable treasury wallet selected at pool initialization.
- Checked arithmetic and explicit errors for invalid amounts, slippage, insufficient liquidity, and invalid pool parameters.

## How Fees Work

For an input amount $A$ and fee $f$ in basis points:

```text
fee          = floor(A * f / 10,000)
pool_input   = A - fee
output       = floor(reserve_out * pool_input / (reserve_in + pool_input))
```

The trader transfers `pool_input` into the pool vault and `fee` into the treasury account for the input token. Treasury balances are kept outside the pool reserves, so LP withdrawals and swap pricing do not count treasury-held fees as pool liquidity. The wallet chosen as treasury controls its associated token accounts through the standard SPL Token program.

## Accounts and Instructions

- `initialize(seed, fee, authority)`: Creates the pool config PDA, LP mint, pool vaults, and treasury token accounts. `authority` is reserved for future pool administration; no update or lock-management instruction is currently exposed.
- `deposit(amount, max_x, max_y)`: Deposits both assets and mints LP tokens. `max_x` and `max_y` cap the required token amounts on subsequent deposits; on the first deposit they are the exact initial amounts.
- `withdraw(amount, min_x, min_y)`: Burns LP tokens and returns the proportional reserves, subject to minimum-output bounds.
- `swap(is_x, amount_in, min_amount_out)`: Swaps the selected input asset, routes its fee to treasury, and checks the minimum output.

Important pool PDAs:

- Config: `PDA(["config", seed.to_le_bytes()])`
- LP mint: `PDA(["lp", config_pubkey])`
- Pool vaults: associated token accounts owned by the config PDA
- Treasury accounts: associated token accounts owned by the treasury wallet

## Repository Layout

```text
programs/amm-video/src/       Anchor program, account state, errors, instructions
programs/amm-video/tests/     LiteSVM integration tests and instruction builders
runbooks/deployment/          Txtx deployment runbook
Anchor.toml                   Anchor program ID and local provider configuration
txtx.yml                      Txtx localnet and devnet environment settings
```

## Build and Test

Prerequisites: Rust toolchain from `rust-toolchain.toml`, Anchor CLI, Solana/Agave tools, and the SBF build tools installed by the Solana toolchain.

From the repository root:

```sh
anchor build
cargo test -p amm-video
```

The integration tests execute the compiled `target/deploy/amm_video.so` program in LiteSVM. They cover initialization and invalid fee rejection; LP creation, deposit bounds, and invalid initial LP issuance; proportional withdrawal and withdrawal slippage; and both swap directions, treasury fee balances, constant-product output, zero input, and swap slippage.

A passing test-run screenshot is included at [`screenshots/tests-passing.png`](screenshots/tests-passing.png).

## Deployment

`Anchor.toml` defaults to localnet and `runbooks/deployment/main.tx` defines the Txtx deployment action. Configure the correct wallet, program keypair, and network before deploying. The on-chain program ID in `Anchor.toml` and `programs/amm-video/src/lib.rs` must match the deployment keypair for a real deployment. The provided Txtx configuration includes localnet and devnet RPC settings; production deployment needs separate operational review and signer management.

## Scope and Limitations

- The implementation uses the classic SPL Token program and six-decimal assets; Token-2022 extensions are not supported.
- The treasury is fixed when the pool is initialized. There is no fee withdrawal instruction because the treasury wallet directly controls its SPL token accounts.
- `authority` and `locked` are stored in pool configuration, but administration instructions to update or lock the pool have not been implemented.
- The tests validate core accounting and rejection paths in LiteSVM; they are not a substitute for a security audit, fuzzing, or mainnet validation.
