# flip demo

shows real-time settlement games from mainnet

```bash
# kv mode - all games
cargo run --example flip

# state mode - single game (use current key from kv output)
VIEW=SettlementGame/state KEY=44514 cargo run --example flip

# list mode - games as array
VIEW=SettlementGame/list cargo run --example flip
```

prints full json with wallets, bets, settlement, payouts
