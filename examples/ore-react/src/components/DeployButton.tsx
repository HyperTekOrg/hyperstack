/**
 * Deploy Button with Automatic Checkpoint
 */

import { useState } from 'react';
import { useWallet, useConnection } from '@solana/wallet-adapter-react';
import { useHyperstack } from 'hyperstack-react';
import { ORE_STREAM_STACK } from 'hyperstack-stacks/ore';
import { Transaction, PublicKey } from '@solana/web3.js';

export function DeployButton({ currentRound, minerData, recentRounds, selectedSquares }: {
  currentRound?: any;
  minerData?: any;
  recentRounds?: any[];
  selectedSquares: number[];
}) {
  const wallet = useWallet();
  const { connection } = useConnection();
  const [amount, setAmount] = useState('0.001');
  const [isProcessing, setIsProcessing] = useState(false);
  const [processingStep, setProcessingStep] = useState<'checkpoint' | 'deploy' | null>(null);
  const [result, setResult] = useState<{ 
    status: 'success' | 'error'; 
    checkpointSignature?: string;
    deploySignature?: string; 
    error?: string;
  } | null>(null);
  
  const stack = useHyperstack(ORE_STREAM_STACK);
  const checkpoint = stack.instructions?.checkpoint?.useMutation();
  const deploy = stack.instructions?.deploy?.useMutation();
  
  // Check if checkpoint is needed
  const needsCheckpoint = 
    minerData?.state?.round_id != null &&
    currentRound?.id?.round_id != null &&
    minerData.state.round_id < currentRound.id.round_id;

  const handleDeploy = async () => {
    if (!wallet.connected || !wallet.publicKey) {
      return;
    }

    if (!currentRound?.id?.round_address || !currentRound?.entropy?.entropy_var_address || selectedSquares.length === 0) {
      return;
    }

    setIsProcessing(true);
    setProcessingStep(needsCheckpoint ? 'checkpoint' : 'deploy');
    setResult(null);

    try {
      const amountLamports = BigInt(Math.floor(parseFloat(amount) * 1e9));
      
      // TO DO: check
      const walletAdapter = {
        publicKey: wallet.publicKey!.toBase58(),
        signAndSend: async (transaction: any) => {
          const tx = new Transaction();
          for (const ix of transaction.instructions) {
            tx.add({
              programId: new PublicKey(ix.programId),
              keys: ix.keys.map((key: any) => ({
                pubkey: new PublicKey(key.pubkey),
                isSigner: key.isSigner,
                isWritable: key.isWritable,
              })),
              data: Buffer.from(ix.data),
            });
          }
          return await wallet.sendTransaction!(tx, connection);
        }
      };

      let checkpointSig: string | undefined;

      // Call checkpoint first if needed
      if (needsCheckpoint) {
        const oldRound = recentRounds?.find(r => r.id?.round_id === minerData.state.round_id);
        
        if (!oldRound?.id?.round_address) {
          throw new Error(`Round ${minerData.state.round_id} not available. Cannot checkpoint.`);
        }

        const checkpointResult = await checkpoint.submit(
          {},
          {
            wallet: walletAdapter,
            accounts: { round: oldRound.id.round_address },
          }
        );
        checkpointSig = checkpointResult.signature;
        setProcessingStep('deploy');
      }

      // Then deploy
      const deployResult = await deploy.submit(
        {
          amount: amountLamports,
          squares: selectedSquares.length,
        },
        {
          wallet: walletAdapter,
          accounts: {
            round: currentRound.id.round_address,
            entropyVar: currentRound.entropy.entropy_var_address!,
          },
        }
      );

      setResult({ 
        status: 'success', 
        checkpointSignature: checkpointSig,
        deploySignature: deployResult.signature 
      });
      
    } catch (err: any) {
      console.error('Deploy failed:', err);
      setResult({ status: 'error', error: err?.message || String(err) });
    } finally {
      setIsProcessing(false);
      setProcessingStep(null);
    }
  };

  return (
    <div className="bg-white dark:bg-stone-800 rounded-2xl p-6 shadow-sm dark:shadow-none dark:ring-1 dark:ring-stone-700">
      <h3 className="text-lg font-semibold text-stone-800 dark:text-stone-100 mb-4">Deploy</h3>
      
      {needsCheckpoint && (
        <div className="mb-4 p-3 bg-amber-50 dark:bg-amber-900/20 border border-amber-200 dark:border-amber-800 rounded-lg text-xs text-amber-700 dark:text-amber-300">
          ℹ️ Will checkpoint round {minerData?.state?.round_id} first
        </div>
      )}
      
      <div className="space-y-4">
        <div>
          <label className="block text-sm font-medium text-stone-600 dark:text-stone-400 mb-2">
            Amount (SOL)
          </label>
          <input
            type="number"
            step="0.001"
            min="0"
            value={amount}
            onChange={(e) => setAmount(e.target.value)}
            disabled={isProcessing}
            className="w-full px-4 py-2.5 bg-stone-50 dark:bg-stone-900 border border-stone-200 dark:border-stone-700 rounded-xl
                     text-stone-800 dark:text-stone-100 placeholder-stone-400 dark:placeholder-stone-500
                     focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent
                     disabled:opacity-50 disabled:cursor-not-allowed transition-all"
            placeholder="0.001"
          />
        </div>

        <div className="text-sm text-stone-600 dark:text-stone-400">
          Selected: <span className="text-blue-600 dark:text-blue-400 font-semibold">{selectedSquares.length}</span> square{selectedSquares.length !== 1 ? 's' : ''}
          {selectedSquares.length > 0 && (
            <span className="ml-2 text-xs text-stone-400 dark:text-stone-500">
              ({selectedSquares.slice(0, 5).join(', ')}{selectedSquares.length > 5 ? '...' : ''})
            </span>
          )}
        </div>

        <button
          onClick={handleDeploy}
          disabled={!wallet.connected || isProcessing || selectedSquares.length === 0 || !amount}
          className="w-full px-6 py-3 bg-blue-600 hover:bg-blue-700 
                   text-white font-semibold rounded-xl shadow-sm
                   disabled:bg-stone-300 dark:disabled:bg-stone-700 
                   disabled:text-stone-500 dark:disabled:text-stone-500
                   disabled:cursor-not-allowed
                   transition-all duration-200 hover:shadow-md active:scale-[0.98]"
        >
          {!wallet.connected ? 'Connect Wallet' : 
           isProcessing ? (needsCheckpoint ? 'Checkpointing + Deploying...' : 'Deploying...') : 
           `Deploy ${amount} SOL`}
        </button>

        {isProcessing && (
          <div className="p-4 bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 rounded-xl text-blue-700 dark:text-blue-400 text-sm">
            ⏳ {processingStep === 'checkpoint' ? 'Checkpointing' : 'Deploying'}...
          </div>
        )}

        {result?.status === 'success' && result.deploySignature && (
          <div className="p-4 bg-green-50 dark:bg-green-900/20 border border-green-200 dark:border-green-800 rounded-xl text-green-700 dark:text-green-400 text-sm">
            <div className="font-semibold mb-2">
              ✅ {result.checkpointSignature ? 'Checkpoint + Deploy' : 'Deploy'} successful!
            </div>
            {result.checkpointSignature && (
              <div className="mb-2">
                <div className="text-xs font-medium mb-1">Checkpoint:</div>
                <a 
                  href={`https://solscan.io/tx/${result.checkpointSignature}`}
                  target="_blank"
                  rel="noopener noreferrer"
                  className="text-xs underline hover:no-underline break-all"
                >
                  {result.checkpointSignature.slice(0, 8)}...{result.checkpointSignature.slice(-8)} →
                </a>
              </div>
            )}
            <div className={result.checkpointSignature ? 'mt-2' : ''}>
              <div className="text-xs font-medium mb-1">Deploy:</div>
              <a 
                href={`https://solscan.io/tx/${result.deploySignature}`}
                target="_blank"
                rel="noopener noreferrer"
                className="text-xs underline hover:no-underline break-all"
              >
                {result.deploySignature.slice(0, 8)}...{result.deploySignature.slice(-8)} →
              </a>
            </div>
            <button
              onClick={() => setResult(null)}
              className="mt-3 w-full text-xs text-green-700 dark:text-green-400 hover:text-green-800 dark:hover:text-green-300 underline"
            >
              Deploy Again
            </button>
          </div>
        )}

        {result?.status === 'error' && result.error && (
          <div className="p-4 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-xl text-red-700 dark:text-red-400 text-sm">
            <div className="font-semibold mb-1">❌ Deploy failed</div>
            <div className="text-xs mb-3 opacity-90">{result.error}</div>
            <button
              onClick={() => setResult(null)}
              className="w-full text-xs text-red-700 dark:text-red-400 hover:text-red-800 dark:hover:text-red-300 underline"
            >
              Try Again
            </button>
          </div>
        )}
      </div>
    </div>
  );
}
