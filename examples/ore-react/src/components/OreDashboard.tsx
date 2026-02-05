import { useHyperstack } from 'hyperstack-react';
import { ORE_STREAM_STACK } from 'hyperstack-stacks/ore';
import { ValidatedOreRoundSchema } from '../schemas/ore-round-validated';
import { BlockGrid } from './BlockGrid';
import { StatsPanel } from './StatsPanel';
import { ConnectionBadge } from './ConnectionBadge';

export function OreDashboard() {
  const { views, isConnected } = useHyperstack(ORE_STREAM_STACK, { url: "ws://localhost:8878" });
  const { data: latestRound } = views.OreRound.latest.useOne({ schema: ValidatedOreRoundSchema });
  console.log(latestRound);
  const { data: treasuryData } = views.OreTreasury.list.useOne();

  return (
    <div className="min-h-screen w-full bg-slate-950 p-6 font-sans text-white relative">
      <div className="max-w-[1400px] mx-auto flex gap-8 flex-wrap">
        <div className="flex-[1_1_700px]">
          <BlockGrid round={latestRound} />
        </div>

        <div className="flex-[0_0_400px]">
          <StatsPanel
            round={latestRound}
            treasuryMotherlode={treasuryData?.state?.motherlode_ui}
            isConnected={isConnected}
          />
        </div>
      </div>

      <ConnectionBadge isConnected={isConnected} />
    </div>
  );
}
