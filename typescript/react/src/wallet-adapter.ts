import { useWallet } from '@solana/wallet-adapter-react';
import type { WalletAdapter } from './types';

export function useHyperstackWallet(): WalletAdapter | undefined {
  const wallet = useWallet();
  
  if (!wallet.connected || !wallet.publicKey) {
    return undefined;
  }
  
  if (!wallet.signTransaction || !wallet.signAllTransactions) {
    return undefined;
  }
  
  return {
    publicKey: wallet.publicKey,
    signTransaction: wallet.signTransaction,
    signAllTransactions: wallet.signAllTransactions,
    connected: wallet.connected,
  };
}
