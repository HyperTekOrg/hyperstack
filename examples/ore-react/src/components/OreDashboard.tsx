import { useHyperstack } from 'hyperstack-react';
import { ORE_STREAM_STACK } from 'hyperstack-stacks/ore';
import { useState, useEffect } from 'react';
import { ValidatedOreRoundSchema, type ValidatedOreRound } from '../schemas/ore-round-validated';

export function OreDashboard() {
  const { views, isConnected } = useHyperstack(ORE_STREAM_STACK, { url: "ws://localhost:8878" });
  const { data: latestRound } = views.OreRound.latest.useOne({ schema: ValidatedOreRoundSchema });
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

      <div className="fixed top-4 right-4 flex items-center gap-2 px-4 py-2 bg-slate-900/90 backdrop-blur-sm rounded-full border border-slate-700/50">
        <div className={`w-2 h-2 rounded-full ${isConnected ? 'bg-emerald-400 shadow-[0_0_8px_rgba(52,211,153,0.5)]' : 'bg-red-500'}`} />
        <span className={`text-xs font-semibold tracking-wide ${isConnected ? 'text-white' : 'text-red-300'}`}>
          {isConnected ? 'CONNECTED' : 'DISCONNECTED'}
        </span>
      </div>
    </div>
  );
}

function BlockGrid({ round }: { round: ValidatedOreRound | undefined }) {
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

function StatsPanel({
  round,
  treasuryMotherlode,
  isConnected,
}: {
  round: ValidatedOreRound | undefined;
  treasuryMotherlode: number | null | undefined;
  isConnected: boolean;
}) {
  const [timeRemaining, setTimeRemaining] = useState<string>('--:--');

  useEffect(() => {
    if (!round) {
      setTimeRemaining('--:--');
      return;
    }

    const updateTimer = () => {
      const now = Math.floor(Date.now() / 1000);
      let expiresAt = round.state.expires_at;

      if (expiresAt > 9999999999) {
        expiresAt = Math.floor(expiresAt / 1000);
      }

      const remaining = Math.max(0, expiresAt - now);

      if (remaining > 300) {
        setTimeRemaining('--:--');
        return;
      }

      const minutes = Math.floor(remaining / 60);
      const seconds = remaining % 60;
      setTimeRemaining(`${String(minutes).padStart(2, '0')}:${String(seconds).padStart(2, '0')}`);
    };

    updateTimer();
    const interval = setInterval(updateTimer, 1000);
    return () => clearInterval(interval);
  }, [round]);

  const motherlodeValue = treasuryMotherlode;

  return (
    <div className="flex flex-col gap-4">
      <div className="flex gap-3">
        <div className="flex-1 bg-gradient-to-br from-violet-900/40 to-slate-900 border-2 border-violet-500/60 rounded-2xl p-5 flex flex-col items-center gap-2 shadow-[0_0_20px_rgba(139,92,246,0.2)]">
          <div className="flex items-center gap-2 text-2xl font-bold text-white">
            <OreIcon />
            <span>{motherlodeValue}</span>
          </div>
          <div className="text-xs text-slate-400 uppercase tracking-wider">Motherlode</div>
        </div>
        <div className="flex-1 bg-slate-900/80 border border-slate-700/50 rounded-2xl p-5 flex flex-col items-center gap-2 transition-colors hover:border-slate-600">
          <div className="text-3xl font-bold text-white tabular-nums">{timeRemaining}</div>
          <div className="text-xs text-slate-400 uppercase tracking-wider">Time remaining</div>
        </div>
      </div>

      <div className="flex gap-3">
        <div className="flex-1 bg-slate-900/80 border border-slate-700/50 rounded-2xl p-5 flex flex-col items-center gap-2 transition-colors hover:border-slate-600">
          <div className="flex items-center gap-2 text-2xl font-bold text-white">
            <SolanaIcon />
            <span>{round ? round.state.total_deployed_ui.toFixed(4) : '0.0000'}</span>
          </div>
          <div className="text-xs text-slate-400 uppercase tracking-wider">Total deployed</div>
        </div>
        <div className="flex-1 bg-slate-900/80 border border-slate-700/50 rounded-2xl p-5 flex flex-col items-center gap-2 transition-colors hover:border-slate-600">
          <div className="flex items-center gap-2 text-2xl font-bold text-white">
            <SolanaIcon />
            <span>0</span>
          </div>
          <div className="text-xs text-slate-400 uppercase tracking-wider">You deployed</div>
        </div>
      </div>

      <div className="flex justify-between items-center px-4 py-3 bg-slate-900/80 rounded-xl border border-slate-700/50">
        <div className="flex items-center gap-4">
          <span className="text-sm text-slate-400">Round #{round?.id.round_id ?? '--'}</span>
          {round && (
            <span className="text-sm text-slate-400">
              {round.state.total_miners} miners
            </span>
          )}
        </div>
      </div>

      {!isConnected && (
        <div className="bg-red-500/10 border border-red-500/30 text-red-300 p-3 rounded-xl text-center text-sm">
          Waiting for connection...
        </div>
      )}
    </div>
  );
}

function MinerIcon() {
  return (
    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" role="img" aria-labelledby="miner-icon-title">
      <title id="miner-icon-title">Miners</title>
      <path d="M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2" />
      <circle cx="9" cy="7" r="4" />
      <path d="M22 21v-2a4 4 0 0 0-3-3.87" />
      <path d="M16 3.13a4 4 0 0 1 0 7.75" />
    </svg>
  );
}

function SolanaIcon({ size = 20 }: { size?: number }) {
  return (
    <svg width={size} height={size} viewBox="0 0 397 311" fill="none" role="img" aria-labelledby="sol-icon-title">
      <title id="sol-icon-title">SOL</title>
      <linearGradient id="sol-gradient" x1="0%" y1="0%" x2="100%" y2="100%">
        <stop offset="0%" stopColor="#00FFA3" />
        <stop offset="100%" stopColor="#DC1FFF" />
      </linearGradient>
      <path
        d="M64.6 237.9c2.4-2.4 5.7-3.8 9.2-3.8h317.4c5.8 0 8.7 7 4.6 11.1l-62.7 62.7c-2.4 2.4-5.7 3.8-9.2 3.8H6.5c-5.8 0-8.7-7-4.6-11.1l62.7-62.7z"
        fill="url(#sol-gradient)"
      />
      <path
        d="M64.6 3.8C67.1 1.4 70.4 0 73.8 0h317.4c5.8 0 8.7 7 4.6 11.1l-62.7 62.7c-2.4 2.4-5.7 3.8-9.2 3.8H6.5c-5.8 0-8.7-7-4.6-11.1L64.6 3.8z"
        fill="url(#sol-gradient)"
      />
      <path
        d="M333.1 120.1c-2.4-2.4-5.7-3.8-9.2-3.8H6.5c-5.8 0-8.7 7-4.6 11.1l62.7 62.7c2.4 2.4 5.7 3.8 9.2 3.8h317.4c5.8 0 8.7-7 4.6-11.1l-62.7-62.7z"
        fill="url(#sol-gradient)"
      />
    </svg>
  );
}

function OreIcon() {
  return (
    <svg width="20" height="20" viewBox="0 0 24 24" fill="none" role="img" aria-labelledby="ore-icon-title">
      <title id="ore-icon-title">ORE</title>
      <circle cx="12" cy="12" r="10" stroke="#a78bfa" strokeWidth="2" fill="none" />
      <circle cx="12" cy="12" r="6" fill="#a78bfa" />
      <circle cx="12" cy="12" r="3" fill="#c4b5fd" />
    </svg>
  );
}
