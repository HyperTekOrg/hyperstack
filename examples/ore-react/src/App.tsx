import { OreDashboard } from './components';
import { HyperstackProvider } from 'hyperstack-react';
import { ThemeProvider } from './hooks/useTheme';
import { useMemo } from 'react';
import { ConnectionProvider, WalletProvider } from '@solana/wallet-adapter-react';
import { WalletModalProvider } from '@solana/wallet-adapter-react-ui';
import { PhantomWalletAdapter } from '@solana/wallet-adapter-wallets';
import '@solana/wallet-adapter-react-ui/styles.css';

export default function App() {
  const endpoint = import.meta.env.VITE_RPC_URL; // add your own RPC URL in a .env file
  
  // Setup wallet adapters
  const wallets = useMemo(
    () => [
      new PhantomWalletAdapter(),
    ],
    []
  );

  return (
    <ConnectionProvider endpoint={endpoint}>
      <WalletProvider wallets={wallets} autoConnect>
        <WalletModalProvider>
          <ThemeProvider>
            <HyperstackProvider autoConnect={true}>
              <OreDashboard />
            </HyperstackProvider>
          </ThemeProvider>
        </WalletModalProvider>
      </WalletProvider>
    </ConnectionProvider>
  );
}
