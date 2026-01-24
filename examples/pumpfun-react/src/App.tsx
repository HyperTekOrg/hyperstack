import { useMemo } from 'react';
import { PumpFunDashboard } from './components/PumpFunDashboard';
import { HyperstackProvider } from 'hyperstack-react';
import { ConnectionProvider, WalletProvider } from '@solana/wallet-adapter-react';
import { WalletAdapterNetwork } from '@solana/wallet-adapter-base';
import { WalletModalProvider } from '@solana/wallet-adapter-react-ui';
import { PhantomWalletAdapter, SolflareWalletAdapter } from '@solana/wallet-adapter-wallets';
import { clusterApiUrl } from '@solana/web3.js';

// Import wallet adapter CSS
import '@solana/wallet-adapter-react-ui/styles.css';

const websocketUrl = import.meta.env.VITE_HYPERSTACK_WS_URL;
const rpcUrl = import.meta.env.VITE_SOLANA_RPC_URL || clusterApiUrl(WalletAdapterNetwork.Mainnet);

export default function App() {
  const wallets = useMemo(
    () => [
      new PhantomWalletAdapter(),
      new SolflareWalletAdapter(),
    ],
    []
  );

  return (
    <ConnectionProvider endpoint={rpcUrl}>
      <WalletProvider wallets={wallets} autoConnect>
        <WalletModalProvider>
          <HyperstackProvider
            websocketUrl={websocketUrl}
            autoConnect={true}
          >
            <PumpFunDashboard />
          </HyperstackProvider>
        </WalletModalProvider>
      </WalletProvider>
    </ConnectionProvider>
  );
}
