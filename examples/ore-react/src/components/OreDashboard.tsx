import { useState } from 'react';
import { useHyperstack } from 'hyperstack-react';
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

export function OreDashboard() {
  const wallet = useWallet();
  const { views, isConnected } = useHyperstack(ORE_STREAM_STACK);
  const { data: latestRound } = views.OreRound.latest.useOne({ schema: ValidatedOreRoundSchema });
  const { data: treasuryData } = views.OreTreasury.list.useOne();

  const { data: minerData } = views.OreMiner.state.use(
    wallet.publicKey ? { authority: wallet.publicKey.toBase58() } : undefined
  );

  const { data: recentRounds } = views.OreRound.list.use({ take: 10 });
  const nowUnix = Math.floor(Date.now() / 1000);
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
          />
        </div>

        <div className="flex-1 min-w-[280px] max-w-md flex flex-col gap-6">
          <StatsPanel
            round={latestRound}
            treasuryMotherlode={treasuryData?.state?.motherlode}
            isConnected={isConnected}
            winnerSquare={statsWinnerSquare}
            winnerRoundId={statsWinnerRoundId}
          />
          <DeployButton 
            currentRound={latestRound}
            minerData={minerData}
            recentRounds={recentRounds}
            selectedSquares={selectedSquares}
          />
        </div>
      </div>

      <ConnectionBadge isConnected={isConnected} />
    </div>
  );
}
