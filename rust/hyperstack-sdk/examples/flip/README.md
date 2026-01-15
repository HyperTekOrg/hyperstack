# flip demo

shows real-time settlement games from mainnet

```bash
# list mode - all games
cargo run --example flip

# state mode - single game (use key from list output)
VIEW=SettlementGame/state KEY=44514 cargo run --example flip
```

prints full json with wallets, bets, settlement, payouts
