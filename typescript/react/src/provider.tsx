import React, { createContext, useContext, useEffect, useMemo, useRef, ReactNode, useSyncExternalStore } from 'react';
import type { ConnectionState } from 'hyperstack-typescript';
import type { HyperstackConfig, NetworkConfig } from './types';
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
