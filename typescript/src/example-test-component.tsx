import React, { useMemo } from 'react';
import { useWallet, useConnection } from '@solana/wallet-adapter-react';
import { HyperstackProvider } from './provider';
import { useHyperstack } from './stack';
import { PUMPFUN_STACK } from './pumpfun-stack';
import { WalletAdapter } from './types';

export function TestApp() {
  const { connection } = useConnection();
  const { publicKey, sendTransaction } = useWallet();

  const walletAdapter: WalletAdapter | undefined = useMemo(() => {
    if (!publicKey || !sendTransaction) return undefined;

    return {
      publicKey: publicKey.toBase58(),
      signAndSend: async (transaction: any) => {
        const signature = await sendTransaction(transaction, connection);
        return signature;
      }
    };
  }, [publicKey, sendTransaction]);

  return (
    <HyperstackProvider
      network="devnet"
      apiKey="test-key"
      autoConnect={true}
      wallet={walletAdapter}
    >
      <TokenDashboard mintPubkey="So11111111111111111111111111111111111111112" />
    </HyperstackProvider>
  );
}

// // Import or generate from your own programs
// import { PUMPFUN_STACK } from "@hyperstack/react";
//
// const pump = useStack(PUMPFUN_STACK);
//
// const { pools } = pump.pools.list.use();
// const { buy } = pump.tx.useMutation();
//
// const buyState = await buy({ mint, amount, maxSol });

function TokenDashboard({ mintPubkey }: { mintPubkey: string }) {
  const pump = useHyperstack(PUMPFUN_STACK);

  const { data: token, isLoading, error: tokenError, refresh } = pump.views.token.state.use(
    { mint: mintPubkey },
    { enabled: !!mintPubkey }
  );

  const { data: pools } = pump.views.pools.list.use();

  const { data: highVolumePools } = pump.views.highVolumePools.list.use();

  const { data: filteredPools } = pump.views.pools.list.use({
    where: { apy: { gte: 10 } },
    limit: 10
  });

  const tokenPrice = pump.store.use(
    (state) => state.entities.get('tokens/state')?.get(mintPubkey)
  );

  const { submit, status, error: txError, signature } = pump.tx.useMutation();

  const handleSwap = async () => {
    await submit(
      pump.tx.swap({
        fromMint: mintPubkey,
        toMint: 'SOL',
        amount: 1000n
      })
    );
  };

  const formattedPrice = token ? pump.helpers.formatPrice(token.price) : null;

  if (isLoading) return <div>Loading token...</div>;
  if (tokenError) return <div>Error: {tokenError.message}</div>;
  if (!token) return <div>Token not found</div>;

  return (
    <div>
      <h1>Token: {token.mint}</h1>
      <p>Supply: {pump.helpers.formatSupply(token.supply)}</p>
      <p>Price: {formattedPrice}</p>
      <p>Holders: {token.holders}</p>
      <button onClick={refresh}>Refresh Token</button>

      <h2>All Pools ({pools?.size ?? 0})</h2>
      {Array.from(pools?.values() ?? []).map(pool => (
        <div key={pool.id}>
          {pool.baseMint}/{pool.quoteMint} - APY: {pool.apy}%
        </div>
      ))}

      <h2>High Volume Pools ({highVolumePools?.size ?? 0})</h2>
      {Array.from(highVolumePools?.values() ?? []).map(pool => (
        <div key={pool.id}>
          {pool.baseMint}/{pool.quoteMint} - Volume: {pool.volume24h?.toString() ?? 'N/A'}
        </div>
      ))}

      <h2>Filtered Pools (APY >= 10%) ({filteredPools?.size ?? 0})</h2>
      {Array.from(filteredPools?.values() ?? []).map(pool => (
        <div key={pool.id}>
          {pool.baseMint}/{pool.quoteMint} - APY: {pool.apy}%
        </div>
      ))}

      <button onClick={handleSwap} disabled={status === 'pending'}>
        {status === 'pending' ? 'Swapping...' : 'Swap'}
      </button>
      {status === 'success' && signature && <p>Success! Tx: {signature}</p>}
      {txError && <p>Error: {txError}</p>}
    </div>
  );
}
