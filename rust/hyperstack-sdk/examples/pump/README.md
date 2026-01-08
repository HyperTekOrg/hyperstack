# pump demos

## new launches
```bash
cargo run --example pump_new
```
detects new token launches, shows creator and initial liquidity

## token trades
```bash
KEY=<mint_address> cargo run --example pump_trades
```
watches specific token, caches last 100 trades, shows trader wallet and SOL amount

