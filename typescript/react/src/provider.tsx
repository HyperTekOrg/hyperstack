import React, { createContext, useContext, useEffect, useMemo, useRef, ReactNode, useSyncExternalStore } from 'react';
import type { ConnectionState } from 'hyperstack-typescript';
import type { HyperstackConfig, NetworkConfig } from './types';
import { createRuntime, HyperstackRuntime } from './runtime';
import { useHyperstackWallet } from './wallet-adapter';
import { WalletProvider, useWallet } from '@solana/wallet-adapter-react';

interface HyperstackContextValue {
  runtime: HyperstackRuntime;
  config: HyperstackConfig;
}

const HyperstackContext = createContext<HyperstackContextValue | null>(null);

function resolveNetworkConfig(network: 'devnet' | 'mainnet' | 'localnet' | NetworkConfig | undefined, websocketUrl?: string): NetworkConfig {
  if (websocketUrl) {
    return {
      name: 'custom',
      websocketUrl
    };
  }

  if (typeof network === 'object') {
    return network;
  }

  if (network === 'mainnet') {
    return {
      name: 'mainnet',
      websocketUrl: 'wss://mainnet.hyperstack.xyz',
    };
  }

  if (network === 'devnet') {
    return {
      name: 'devnet',
      websocketUrl: 'ws://localhost:8080',
    };
  }

  if (network === 'localnet') {
    return {
      name: 'localnet',
      websocketUrl: 'ws://localhost:8080',
    };
  }

  throw new Error('Must provide either network or websocketUrl');
}

function InternalHyperstackProvider({
  children,
  ...config
}: HyperstackConfig & {
  children: ReactNode;
}) {
  const wallet = useHyperstackWallet();
  
  const networkConfig = useMemo(() => {
    try {
      return resolveNetworkConfig(config.network, config.websocketUrl);
    } catch (error) {
      console.error('[Hyperstack] Invalid network configuration:', error);
      throw error;
    }
  }, [config.network, config.websocketUrl]);

  const runtimeRef = useRef<HyperstackRuntime | null>(null);

  if (!runtimeRef.current) {
    try {
      runtimeRef.current = createRuntime({
        ...config,
        websocketUrl: networkConfig.websocketUrl,
        network: networkConfig,
        wallet
      });
    } catch (error) {
      console.error('[Hyperstack] Failed to create runtime:', error);
      throw error;
    }
  }

  const runtime = runtimeRef.current;
  
  useEffect(() => {
    runtime.wallet = wallet;
  }, [wallet, runtime]);

  const isMountedRef = useRef(true);

  useEffect(() => {
    isMountedRef.current = true;

    if (config.autoConnect !== false) {
      try {
        runtime.connection.connect();
      } catch (error) {
        console.error('[Hyperstack] Failed to auto-connect:', error);
      }
    }

    return () => {
      isMountedRef.current = false;
      setTimeout(() => {
        if (!isMountedRef.current) {
          try {
            runtime.subscriptionRegistry.clear();
            runtime.connection.disconnect();
          } catch (error) {
            console.error('[Hyperstack] Failed to disconnect:', error);
          }
        }
      }, 100);
    };
  }, [runtime, config.autoConnect]);

  const value: HyperstackContextValue = {
    runtime,
    config: { ...config, network: networkConfig }
  };

  return (
    <HyperstackContext.Provider value={value}>
      {children}
    </HyperstackContext.Provider>
  );
}

export function HyperstackProvider({
  children,
  wallets,
  ...config
}: HyperstackConfig & {
  children: ReactNode;
}) {
  let hasWalletContext = false;
  
  try {
    useWallet();
    hasWalletContext = true;
  } catch {
    hasWalletContext = false;
  }
  
  if (!hasWalletContext && wallets && wallets.length > 0) {
    return (
      <WalletProvider wallets={wallets} autoConnect>
        <InternalHyperstackProvider {...config}>
          {children}
        </InternalHyperstackProvider>
      </WalletProvider>
    );
  }
  
  return (
    <InternalHyperstackProvider {...config}>
      {children}
    </InternalHyperstackProvider>
  );
}

export function useHyperstackContext() {
  const context = useContext(HyperstackContext);
  if (!context) {
    throw new Error('useHyperstackContext must be used within HyperstackProvider');
  }
  return context;
}

export function useConnectionState(): ConnectionState {
  const { runtime } = useHyperstackContext();
  return useSyncExternalStore(
    (callback) => {
      const unsubscribe = runtime.zustandStore.subscribe(callback);
      return unsubscribe;
    },
    () => runtime.zustandStore.getState().connectionState
  );
}

export function useView<T>(viewPath: string): T[] {
  const { runtime } = useHyperstackContext();
  return useSyncExternalStore(
    (callback) => runtime.zustandStore.subscribe(callback),
    () => {
      const viewMap = runtime.zustandStore.getState().entities.get(viewPath);
      if (!viewMap) return [];
      return Array.from(viewMap.values()) as T[];
    }
  );
}

export function useEntity<T>(viewPath: string, key: string): T | null {
  const { runtime } = useHyperstackContext();
  return useSyncExternalStore(
    (callback) => runtime.zustandStore.subscribe(callback),
    () => {
      const viewMap = runtime.zustandStore.getState().entities.get(viewPath);
      if (!viewMap) return null;
      const value = viewMap.get(key);
      return value !== undefined ? (value as T) : null;
    }
  );
}
