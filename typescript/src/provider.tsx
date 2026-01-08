import React, { createContext, useContext, useEffect, useMemo, useRef, ReactNode } from 'react';
import { HyperstackConfig, NetworkConfig } from './types';
import { createRuntime, HyperstackRuntime } from './runtime';

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

export function HyperstackProvider({
  children,
  ...config
}: HyperstackConfig & {
  children: ReactNode;
}) {
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
        network: networkConfig
      });
    } catch (error) {
      console.error('[Hyperstack] Failed to create runtime:', error);
      throw error;
    }
  }
  
  const runtime = runtimeRef.current;

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

export function useHyperstackContext() {
  const context = useContext(HyperstackContext);
  if (!context) {
    throw new Error('useHyperstackContext must be used within HyperstackProvider');
  }
  return context;
}
