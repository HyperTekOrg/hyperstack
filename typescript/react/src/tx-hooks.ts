import { useState } from 'react';
import { Transaction, TransactionInstruction } from '@solana/web3.js';
import { HyperstackRuntime } from './runtime';
import { TransactionDefinition, UseMutationReturn } from './types';

export function createTxMutationHook(
  runtime: HyperstackRuntime,
  transactions?: Record<string, TransactionDefinition>
) {
  return function useMutation(): UseMutationReturn {
    const [status, setStatus] = useState<'idle' | 'pending' | 'success' | 'error'>('idle');
    const [error, setError] = useState<string | undefined>();
    const [signature, setSignature] = useState<string | undefined>();

    const submit = async (
      instructionOrBuilder: TransactionInstruction | TransactionInstruction[]
    ): Promise<string> => {
      setStatus('pending');
      setError(undefined);
      setSignature(undefined);

      try {
        if (!instructionOrBuilder) {
          throw new Error('Transaction instruction is required');
        }

        if (!runtime.wallet) {
          throw new Error('Wallet not connected. Please provide a wallet adapter to HyperstackProvider.');
        }

        if (!runtime.solanaConnection) {
          throw new Error('Solana connection not initialized. Please provide rpcUrl or connection to HyperstackProvider.');
        }

        const tx = new Transaction();
        const { blockhash } = await runtime.solanaConnection.getLatestBlockhash();
        tx.recentBlockhash = blockhash;
        tx.feePayer = runtime.wallet.publicKey!;

        const instructions = Array.isArray(instructionOrBuilder) 
          ? instructionOrBuilder 
          : [instructionOrBuilder];
        tx.add(...instructions);

        const signedTx = await runtime.wallet.signTransaction(tx);

        const txSignature = await runtime.solanaConnection.sendRawTransaction(
          signedTx.serialize(),
          { skipPreflight: false }
        );

        await runtime.solanaConnection.confirmTransaction(txSignature);

        setSignature(txSignature);
        setStatus('success');

        // Auto-refresh views if configured
        if (transactions) {
          for (const txDef of Object.values(transactions)) {
            if (txDef.refresh) {
              for (const refreshTarget of txDef.refresh) {
                try {
                  const key = typeof refreshTarget.key === 'function'
                    ? refreshTarget.key()
                    : refreshTarget.key;
                  runtime.subscribe(refreshTarget.view, key);
                } catch (err) {
                  console.error('[Hyperstack] Error refreshing view:', err);
                }
              }
            }
          }
        }

        return txSignature;
      } catch (err) {
        const errorMessage = err instanceof Error ? err.message : 'Transaction failed';
        console.error('[Hyperstack] Transaction error:', errorMessage, err);
        setStatus('error');
        setError(errorMessage);
        throw err;
      }
    };

    const reset = () => {
      setStatus('idle');
      setError(undefined);
      setSignature(undefined);
    };

    return {
      submit,
      status,
      error,
      signature,
      reset
    };
  };
}
