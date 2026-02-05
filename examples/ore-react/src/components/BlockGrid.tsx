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
    <div className="grid grid-cols-5 gap-2 max-w-[700px]">
      {blocks.map((block) => (
        <div
          key={block.id}
          className={`
            bg-slate-900/80 border rounded-xl p-3 flex flex-col gap-5 min-h-[100px]
            transition-all duration-300 hover:bg-slate-800/80 hover:border-slate-600
            ${block.isWinner
              ? 'border-2 border-violet-500 shadow-[0_0_25px_rgba(139,92,246,0.4)] animate-pulse-glow'
              : 'border-slate-700/50'
            }
          `}
        >
          <div className="flex justify-between items-center">
            <span className="text-slate-500 text-sm font-medium">#{block.id}</span>
            <div className="flex items-center gap-1 text-slate-500 text-sm">
              <span className="text-slate-300">{block.minerCount}</span>
              <MinerIcon />
            </div>
          </div>
          <div className="flex items-center justify-center gap-1.5 text-base font-semibold text-white">
            <SolanaIcon size={14} />
            <span>{Number(block.deployedUi).toFixed(4)}</span>
          </div>
        </div>
      ))}
    </div>
  );
}
