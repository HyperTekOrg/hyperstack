import { useHyperstack } from 'hyperstack-react';
import { ORE_STREAM_STACK } from 'hyperstack-stacks/ore';
import { ValidatedOreRoundSchema } from '../schemas/ore-round-validated';
import { BlockGrid } from './BlockGrid';
import { StatsPanel } from './StatsPanel';
import { ConnectionBadge } from './ConnectionBadge';
import { ThemeToggle } from './ThemeToggle';

export function OreDashboard() {
  const { views, isConnected } = useHyperstack(ORE_STREAM_STACK, { url: "ws://localhost:8878" });
  const { data: latestRound } = views.OreRound.latest.useOne({ schema: ValidatedOreRoundSchema });
  const { data: treasuryData } = views.OreTreasury.list.useOne();

  return (
    <div className="min-h-screen w-full bg-stone-100 dark:bg-stone-900 p-8 md:p-12 font-sans text-stone-900 dark:text-stone-100 relative transition-colors">
      <div className="max-w-6xl mx-auto">
        <header className="mb-10 flex items-start justify-between">
          <div>
            <h1 className="text-2xl font-semibold text-stone-800 dark:text-stone-100">Ore Mining</h1>
            <p className="text-stone-500 dark:text-stone-400 mt-1">Live ORE rounds powered by <a href="https://docs.usehyperstack.com" target="_blank" rel="noreferrer" className="underline">Hyperstack</a></p>
          </div>
          <ThemeToggle />
        </header>

        <div className="flex gap-10 flex-wrap items-start">
          <div className="flex-1 min-w-[500px]">
            <BlockGrid round={latestRound} />
          </div>

          <div className="w-80 flex-shrink-0">
            <StatsPanel
              round={latestRound}
              treasuryMotherlode={treasuryData?.state?.motherlode_ui}
              isConnected={isConnected}
            />
          </div>
        </div>
      </div>

      <ConnectionBadge isConnected={isConnected} />
    </div>
  );
}
