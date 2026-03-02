import { useState, useEffect } from 'react';
import { useHyperstack, type UseMutationResult } from 'hyperstack-react';
import { ORE_STREAM_STACK } from 'hyperstack-stacks/ore';
import { ValidatedOreRoundSchema } from '../schemas/ore-round-validated';
import { BlockGrid } from './BlockGrid';
import { StatsPanel } from './StatsPanel';
import { ConnectionBadge } from './ConnectionBadge';
import { ThemeToggle } from './ThemeToggle';
import { DeployButton } from './DeployButton';
import { getStatsWinner } from './winner-utils';
import { WalletMultiButton } from '@solana/wallet-adapter-react-ui';
import { useWallet } from '@solana/wallet-adapter-react';
import type { Miner } from 'hyperstack-stacks/ore';

// The miner_snapshot field arrives at runtime as an account wrapper, but the
// package TypeScript type incorrectly types it as `Miner` directly.
type MinerSnapshot = { data: Miner; account_address: string; signature: string; slot: number; timestamp: number };

export function OreDashboard() {
  const wallet = useWallet();
  const { views, isConnected, instructions } = useHyperstack(ORE_STREAM_STACK);
  const checkpoint: UseMutationResult | undefined = instructions?.checkpoint?.useMutation();
  const deploy: UseMutationResult | undefined = instructions?.deploy?.useMutation();
  const { data: latestRound } = views.OreRound.latest.useOne({ schema: ValidatedOreRoundSchema });
  const { data: treasuryData } = views.OreTreasury.list.useOne();

  const { data: minerData } = views.OreMiner.state.use(
    { authority: wallet.publicKey?.toBase58() ?? "" },
    { enabled: !!wallet.publicKey }
  );

  const snapshot = (minerData?.miner_snapshot as MinerSnapshot | null | undefined)?.data;

  const currentRoundId = snapshot?.round_id;
  const myDeployed: number[] | undefined =
    currentRoundId != null && currentRoundId === latestRound?.id?.round_id
      ? snapshot?.deployed
      : undefined;

  const { data: recentRounds } = views.OreRound.list.use({ take: 10 });
  const [nowUnix, setNowUnix] = useState(() => Math.floor(Date.now() / 1000));
  useEffect(() => {
    const interval = setInterval(() => setNowUnix(Math.floor(Date.now() / 1000)), 1000);
    return () => clearInterval(interval);
  }, []);
  const { winnerSquare: statsWinnerSquare, winnerRoundId: statsWinnerRoundId } =
    getStatsWinner(latestRound, recentRounds, nowUnix);
  
  const [selectedSquares, setSelectedSquares] = useState<number[]>([]);

  const handleSquareClick = (squareId: number) => {
    setSelectedSquares(prev =>
      prev.includes(squareId)
        ? prev.filter(id => id !== squareId)
        : [...prev, squareId]
    );
  };

  return (
    <div className="h-screen w-full bg-stone-100 dark:bg-stone-900 p-6 font-sans text-stone-900 dark:text-stone-100 relative transition-colors overflow-hidden flex flex-col">
      <header className="flex items-start justify-between mb-6 flex-shrink-0">
        <div>
          <h1 className="text-xl font-semibold text-stone-800 dark:text-stone-100">Ore Mining</h1>
          <p className="text-stone-500 dark:text-stone-400 text-sm">Live ORE rounds powered by <a href="https://docs.usehyperstack.com" target="_blank" rel="noreferrer" className="underline">Hyperstack</a></p>
        </div>
        <div className="flex items-center gap-3">
          <WalletMultiButton />
          <ThemeToggle />
        </div>
      </header>

      <div className="flex gap-6 flex-1 min-h-0">
        <div className="flex-shrink-0">
          <BlockGrid 
            round={latestRound}
            selectedSquares={selectedSquares}
            onSquareClick={handleSquareClick}
            winnerSquare={statsWinnerSquare}
            myDeployedSquares={
              myDeployed
                ?.map((v, i) => (v > 0 ? i : -1))
                .filter((i): i is number => i >= 0)
            }
          />
        </div>

        <div className="flex-1 min-w-[280px] max-w-md flex flex-col gap-6">
          <StatsPanel
            round={latestRound}
            treasuryMotherlode={treasuryData?.state?.motherlode}
            isConnected={isConnected}
            winnerSquare={statsWinnerSquare}
            winnerRoundId={statsWinnerRoundId}
            minerDeployedThisRoundSol={
              myDeployed != null
                ? myDeployed.reduce((a, b) => a + b, 0) / 1_000_000_000
                : undefined
            }
          />
          <DeployButton 
            currentRound={latestRound}
            minerData={minerData}
            recentRounds={recentRounds}
            selectedSquares={selectedSquares}
            onClearSquares={() => setSelectedSquares([])}
            checkpoint={checkpoint}
            deploy={deploy}
          />
        </div>
      </div>

      <ConnectionBadge isConnected={isConnected} />
    </div>
  );
}
