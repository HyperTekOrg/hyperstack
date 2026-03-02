import type { ValidatedOreRound } from '../schemas/ore-round-validated';
import { MinerIcon, SolanaIcon } from './icons';

interface BlockGridProps {
  round: ValidatedOreRound | undefined;
  selectedSquares: number[];
  onSquareClick: (squareId: number) => void;
  winnerSquare?: number | null;
  myDeployedSquares?: number[];
}

export function BlockGrid({ round, selectedSquares, onSquareClick, winnerSquare, myDeployedSquares = [] }: BlockGridProps) {
  const blocks = round
    ? round.state.deployed_per_square_ui.map((deployedUi, i) => ({
      id: i + 1,
      index: i,
      minerCount: round.state.count_per_square[i],
      deployedUi,
    }))
    : Array.from({ length: 25 }, (_, i) => ({
      id: i + 1,
      index: i,
      minerCount: 0,
      deployedUi: 0,
    }));

  return (
    <div 
      className="grid grid-cols-5 gap-2"
      style={{ 
        height: 'calc(100vh - 120px)',
        width: 'calc((100vh - 120px - 4 * 0.5rem) / 5 * 5 + 4 * 0.5rem)'
      }}
    >
      {blocks.map((block) => {
        const isWinner = winnerSquare === block.index;
        const isDeployed = myDeployedSquares.includes(block.index);
        const isSelected = selectedSquares.includes(block.id);

        const squareClass = isDeployed
          ? 'bg-emerald-50 dark:bg-emerald-900/30 ring-2 ring-emerald-500 shadow-lg'
          : isSelected
          ? 'bg-blue-50 dark:bg-blue-900/30 ring-2 ring-blue-500 shadow-lg'
          : isWinner
          ? 'bg-amber-50/70 dark:bg-amber-900/20 shadow-sm dark:shadow-none dark:ring-1 dark:ring-amber-800/50'
          : 'shadow-sm dark:shadow-none dark:ring-1 dark:ring-stone-700';

        return (
          <button
            key={block.id}
            onClick={() => onSquareClick(block.id)}
            style={{ aspectRatio: '1' }}
            className={`
              relative bg-white dark:bg-stone-800 rounded-2xl p-4 flex flex-col justify-between
              transition-all duration-200 hover:shadow-md dark:hover:bg-stone-700
              cursor-pointer hover:scale-[1.02] active:scale-[0.98]
              ${squareClass}
            `}
          >
            <div className="flex justify-between items-start">
              <span className="text-stone-400 dark:text-stone-500 text-sm font-medium">{block.id}</span>
              <div className="flex items-center gap-1.5 text-stone-400 dark:text-stone-500">
                <span className="text-sm">{block.minerCount}</span>
                <MinerIcon />
              </div>
            </div>
            <div className="flex items-center gap-2 text-xl font-semibold text-stone-800 dark:text-stone-100">
              <SolanaIcon size={18} />
              <span>{Number(block.deployedUi).toFixed(4)}</span>
            </div>
            {isWinner && (
              <div className="absolute bottom-2 right-2 text-amber-400 text-sm leading-none opacity-80">👑</div>
            )}
            {isDeployed && (
              <div className="absolute bottom-2 right-2 w-5 h-5 bg-emerald-500 text-white rounded-full flex items-center justify-center text-xs font-bold">✓</div>
            )}
            {!isDeployed && isSelected && (
              <div className="absolute bottom-2 right-2 w-5 h-5 bg-blue-500 text-white rounded-full flex items-center justify-center text-xs font-bold">✓</div>
            )}
          </button>
        );
      })}
    </div>
  );
}
