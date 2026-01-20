import { useState } from 'react';
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

    const submit = async (instructionOrTx: any): Promise<string> => {
      setStatus('pending');
      setError(undefined);
      setSignature(undefined);

      try {
        if (!instructionOrTx) {
          throw new Error('Transaction instruction or transaction object is required');
        }

        if (!runtime.wallet) {
          throw new Error('Wallet not connected. Please provide a wallet adapter to HyperstackProvider.');
        }

        let txSignature: string;
        let instructionsToRefresh: Array<{ instruction: string; params: any }> = [];

        if (Array.isArray(instructionOrTx)) {
          txSignature = await runtime.wallet.signAndSend(instructionOrTx);
          instructionsToRefresh = instructionOrTx.filter(
            inst => inst && typeof inst === 'object' && inst.instruction && inst.params !== undefined
          );
        } else if (
          typeof instructionOrTx === 'object' &&
          instructionOrTx.instruction &&
          instructionOrTx.params !== undefined
        ) {
          txSignature = await runtime.wallet.signAndSend(instructionOrTx);
          instructionsToRefresh = [instructionOrTx];
        } else {
          txSignature = await runtime.wallet.signAndSend(instructionOrTx);
        }

        setSignature(txSignature);
        setStatus('success');

        if (transactions && instructionsToRefresh.length > 0) {
          for (const inst of instructionsToRefresh) {
            const txDef = transactions[inst.instruction];
            if (txDef?.refresh) {
              for (const refreshTarget of txDef.refresh) {
                try {
                  const key = typeof refreshTarget.key === 'function'
                    ? refreshTarget.key(inst.params)
                    : refreshTarget.key;

                  runtime.subscribe(refreshTarget.view, key);
                } catch (err) {
                  console.error('[Hyperstack] Error refreshing view after transaction:', err);
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
