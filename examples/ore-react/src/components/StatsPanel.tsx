import { useState, useEffect } from 'react';
import type { ValidatedOreRound } from '../schemas/ore-round-validated';
import { OreIcon, SolanaIcon } from './icons';

interface StatsPanelProps {
  round: ValidatedOreRound | undefined;
  treasuryMotherlode: number | null | undefined;
  isConnected: boolean;
}

export function StatsPanel({ round, treasuryMotherlode, isConnected }: StatsPanelProps) {
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

  return (
    <div className="flex flex-col gap-4">
      <div className="flex gap-3">
        <div className="flex-1 bg-gradient-to-br from-violet-900/40 to-slate-900 border-2 border-violet-500/60 rounded-2xl p-5 flex flex-col items-center gap-2 shadow-[0_0_20px_rgba(139,92,246,0.2)]">
          <div className="flex items-center gap-2 text-2xl font-bold text-white">
            <OreIcon />
            <span>{treasuryMotherlode}</span>
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
