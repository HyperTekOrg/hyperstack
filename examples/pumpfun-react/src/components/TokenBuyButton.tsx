import { useState } from 'react';
import { useWallet } from '@solana/wallet-adapter-react';
import { WalletMultiButton } from '@solana/wallet-adapter-react-ui';
import { useHyperstack } from 'hyperstack-react';
import { PUMPFUNTOKEN_STACK } from 'hyperstack-stacks/pumpfun';
import {
  createBuyInstruction,
  findAssociatedBondingCurveATA,
  findGlobalPDA,
  findCreatorVaultPDA,
  findEventAuthorityPDA,
  findGlobalVolumeAccumulatorPDA,
  findUserVolumeAccumulatorPDA,
  findFeeConfigPDA,
} from 'hyperstack-stacks/pumpfun';
import { PublicKey, SystemProgram } from '@solana/web3.js';
import { getAssociatedTokenAddressSync, TOKEN_PROGRAM_ID, ASSOCIATED_TOKEN_PROGRAM_ID } from '@solana/spl-token';

interface TokenBuyButtonProps {
  mint: string;
  bondingCurve: string;
  tokenSymbol?: string;
}

export function TokenBuyButton({ mint, bondingCurve, tokenSymbol }: TokenBuyButtonProps) {
  const { publicKey } = useWallet();
  const stack = useHyperstack(PUMPFUNTOKEN_STACK);
  const { submit, status, signature, error } = stack.tx.useMutation();
  
  const [amount, setAmount] = useState('1000000'); // 1M tokens
  const [maxSol, setMaxSol] = useState('0.01'); // 0.01 SOL max

  if (!publicKey) {
    return (
      <div className="flex flex-col gap-2">
        <WalletMultiButton className="!bg-purple-600 hover:!bg-purple-700 !rounded-lg" />
        <p className="text-xs text-gray-500">Connect wallet to buy tokens</p>
      </div>
    );
  }

  const handleBuy = async () => {
    try {
      const mintPubkey = new PublicKey(mint);
      const bondingCurvePubkey = new PublicKey(bondingCurve);
      
      // Derive all required PDAs
      const [global] = findGlobalPDA();
      const [associatedBondingCurve] = findAssociatedBondingCurveATA(bondingCurvePubkey, mintPubkey);
      const [eventAuthority] = findEventAuthorityPDA();
      const [globalVolumeAccumulator] = findGlobalVolumeAccumulatorPDA();
      const [userVolumeAccumulator] = findUserVolumeAccumulatorPDA(publicKey);
      const [feeConfig] = findFeeConfigPDA();
      
      // Get user's associated token account
      const associatedUser = getAssociatedTokenAddressSync(
        mintPubkey,
        publicKey,
        false,
        TOKEN_PROGRAM_ID,
        ASSOCIATED_TOKEN_PROGRAM_ID
      );
      
      // For this example, we'll use placeholder addresses for fee recipient and creator vault
      // In production, these would be fetched from the global and bonding curve accounts
      const feeRecipient = new PublicKey("CebN5WGQ4jvEPvsVU4EoHEpgzq1VV7AbicfhtW4xC9iM"); // Placeholder
      const [creatorVault] = findCreatorVaultPDA(publicKey); // Using user as creator for now
      
      const instruction = createBuyInstruction(
        {
          global,
          feeRecipient,
          mint: mintPubkey,
          bondingCurve: bondingCurvePubkey,
          associatedBondingCurve,
          associatedUser,
          user: publicKey,
          systemProgram: SystemProgram.programId,
          tokenProgram: TOKEN_PROGRAM_ID,
          creatorVault,
          eventAuthority,
          globalVolumeAccumulator,
          userVolumeAccumulator,
          feeConfig,
        },
        {
          amount: BigInt(amount),
          maxSolCost: BigInt(Math.floor(parseFloat(maxSol) * 1_000_000_000)),
          trackVolume: true,
        }
      );

      const txSignature = await submit(instruction);
      console.log('Buy transaction successful:', txSignature);
    } catch (err) {
      console.error('Buy failed:', err);
    }
  };

  return (
    <div className="flex flex-col gap-3 p-4 bg-white rounded-lg border">
      <div className="flex items-center justify-between">
        <h4 className="font-medium text-gray-800">Buy {tokenSymbol || 'Token'}</h4>
        <WalletMultiButton className="!bg-gray-200 hover:!bg-gray-300 !text-gray-800 !text-xs !py-1 !px-2 !rounded" />
      </div>

      <div className="flex flex-col gap-2">
        <div>
          <label className="text-xs text-gray-600">Amount (tokens)</label>
          <input
            type="text"
            value={amount}
            onChange={(e) => setAmount(e.target.value.replace(/[^0-9]/g, ''))}
            className="w-full px-3 py-2 text-sm border rounded-md focus:outline-none focus:ring-2 focus:ring-purple-500"
            placeholder="1000000"
          />
        </div>

        <div>
          <label className="text-xs text-gray-600">Max SOL to spend</label>
          <input
            type="text"
            value={maxSol}
            onChange={(e) => setMaxSol(e.target.value.replace(/[^0-9.]/g, ''))}
            className="w-full px-3 py-2 text-sm border rounded-md focus:outline-none focus:ring-2 focus:ring-purple-500"
            placeholder="0.01"
          />
        </div>
      </div>

      <button
        onClick={handleBuy}
        disabled={status === 'pending'}
        className="w-full px-4 py-2 bg-green-600 hover:bg-green-700 disabled:bg-gray-400 text-white font-medium rounded-md transition-colors text-sm"
      >
        {status === 'pending' ? 'Buying...' : 'Buy Tokens'}
      </button>

      {status === 'success' && signature && (
        <div className="text-xs text-green-600 break-all">
          ✓ Success! 
          <a 
            href={`https://solscan.io/tx/${signature}`}
            target="_blank"
            rel="noopener noreferrer"
            className="underline ml-1"
          >
            View on Solscan
          </a>
        </div>
      )}

      {status === 'error' && error && (
        <div className="text-xs text-red-600 break-all">
          ✗ Error: {error}
        </div>
      )}

      <div className="text-xs text-gray-500">
        <p>• Wallet: {publicKey.toBase58().slice(0, 4)}...{publicKey.toBase58().slice(-4)}</p>
        <p>• Mint: {mint.slice(0, 4)}...{mint.slice(-4)}</p>
      </div>
    </div>
  );
}
