import type { ValidatedOreRound } from '../schemas/ore-round-validated';
import { MinerIcon, SolanaIcon } from './icons';

interface BlockGridProps {
  round: ValidatedOreRound | undefined;
}

export function BlockGrid({ round }: BlockGridProps) {
  const blocks = round
    ? round.state.deployed_per_square_ui.map((deployedUi, i) => ({
      id: i + 1,
      minerCount: round.state.count_per_square[i],
      deployedUi,
      isWinner: round.results?.winning_square === i,
    }))
    : Array.from({ length: 25 }, (_, i) => ({
      id: i + 1,
      minerCount: 0,
      deployedUi: 0,
      isWinner: false,
    }));

  return (
    <div className="grid grid-cols-5 gap-3">
      {blocks.map((block) => (
        <div
          key={block.id}
          className={`
            bg-white dark:bg-stone-800 rounded-2xl p-4 flex flex-col justify-between min-h-[110px]
            transition-all duration-200 hover:shadow-md dark:hover:bg-stone-750
            ${block.isWinner
              ? 'bg-amber-50 dark:bg-amber-900/30 ring-2 ring-amber-400 shadow-lg'
              : 'shadow-sm dark:shadow-none dark:ring-1 dark:ring-stone-700'
            }
          `}
        >
          <div className="flex justify-between items-start">
            <span className="text-stone-400 dark:text-stone-500 text-xs font-medium">{block.id}</span>
            <div className="flex items-center gap-1 text-stone-400 dark:text-stone-500">
              <span className="text-xs">{block.minerCount}</span>
              <MinerIcon />
            </div>
          </div>
          <div className="flex items-center gap-1.5 text-lg font-semibold text-stone-800 dark:text-stone-100">
            <SolanaIcon size={14} />
            <span>{Number(block.deployedUi).toFixed(4)}</span>
          </div>
        </div>
      ))}
    </div>
  );
}
