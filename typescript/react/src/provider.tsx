import React, { createContext, useContext, useEffect, useRef, ReactNode, useSyncExternalStore, useState, useCallback } from 'react';
import { HyperStack, type ConnectionState, type StackDefinition } from 'hyperstack-typescript';
import type { HyperstackConfig, NetworkConfig } from './types';

interface ClientEntry {
  client: HyperStack<any>;
  disconnect: () => void;
}

interface HyperstackContextValue {
  getOrCreateClient: <TStack extends StackDefinition>(stack: TStack) => Promise<HyperStack<TStack>>;
  getClient: <TStack extends StackDefinition>(stack: TStack) => HyperStack<TStack> | null;
  config: {
    websocketUrl: string;
    autoConnect?: boolean;
    reconnectIntervals?: number[];
    maxReconnectAttempts?: number;
    maxEntriesPerView?: number | null;
  };
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
  fallback = null,
  ...config
}: HyperstackConfig & {
  children: ReactNode;
  fallback?: ReactNode;
}) {
  const networkConfig = resolveNetworkConfig(config.network, config.websocketUrl);
  const clientsRef = useRef<Map<string, ClientEntry>>(new Map());
  const connectingRef = useRef<Map<string, Promise<HyperStack<any>>>>(new Map());

  const getOrCreateClient = useCallback(async <TStack extends StackDefinition>(stack: TStack): Promise<HyperStack<TStack>> => {
    const existing = clientsRef.current.get(stack.name);
    if (existing) {
      return existing.client as HyperStack<TStack>;
    }

    const connecting = connectingRef.current.get(stack.name);
    if (connecting) {
      return connecting as Promise<HyperStack<TStack>>;
    }

    const connectionPromise = HyperStack.connect(networkConfig.websocketUrl, {
      stack,
      autoReconnect: config.autoConnect,
      reconnectIntervals: config.reconnectIntervals,
      maxReconnectAttempts: config.maxReconnectAttempts,
      maxEntriesPerView: config.maxEntriesPerView,
    }).then((client) => {
      clientsRef.current.set(stack.name, {
        client,
        disconnect: () => client.disconnect()
      });
      connectingRef.current.delete(stack.name);
      return client;
    });

    connectingRef.current.set(stack.name, connectionPromise);
    return connectionPromise as Promise<HyperStack<TStack>>;
  }, [networkConfig.websocketUrl, config.autoConnect, config.reconnectIntervals, config.maxReconnectAttempts, config.maxEntriesPerView]);

  const getClient = useCallback(<TStack extends StackDefinition>(stack: TStack): HyperStack<TStack> | null => {
    const entry = clientsRef.current.get(stack.name);
    return entry ? (entry.client as HyperStack<TStack>) : null;
  }, []);

  useEffect(() => {
    return () => {
      clientsRef.current.forEach((entry) => {
        entry.disconnect();
      });
      clientsRef.current.clear();
      connectingRef.current.clear();
    };
  }, []);

  const value: HyperstackContextValue = {
    getOrCreateClient,
    getClient,
    config: {
      websocketUrl: networkConfig.websocketUrl,
      autoConnect: config.autoConnect,
      reconnectIntervals: config.reconnectIntervals,
      maxReconnectAttempts: config.maxReconnectAttempts,
      maxEntriesPerView: config.maxEntriesPerView,
    }
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

export function useConnectionState(stack: StackDefinition): ConnectionState {
  const { getClient } = useHyperstackContext();
  const client = getClient(stack);
  
  return useSyncExternalStore(
    (callback) => {
      if (!client) return () => {};
      return client.onConnectionStateChange(callback);
    },
    () => client?.connectionState ?? 'disconnected'
  );
}

export function useView<T>(stack: StackDefinition, viewPath: string): T[] {
  const { getClient } = useHyperstackContext();
  const client = getClient(stack);
  
  return useSyncExternalStore(
    (callback) => {
      if (!client) return () => {};
      return client.store.onUpdate(callback);
    },
    () => {
      if (!client) return [];
      const data = client.store.getAll(viewPath);
      return data as T[];
    }
  );
}

export function useEntity<T>(stack: StackDefinition, viewPath: string, key: string): T | null {
  const { getClient } = useHyperstackContext();
  const client = getClient(stack);
  
  return useSyncExternalStore(
    (callback) => {
      if (!client) return () => {};
      return client.store.onUpdate(callback);
    },
    () => {
      if (!client) return null;
      const data = client.store.get(viewPath, key);
      return data as T | null;
    }
  );
}