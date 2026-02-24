import { useState, useEffect } from 'react';
import type { ValidatedOreRound } from '../schemas/ore-round-validated';
import { OreIcon, SolanaIcon } from './icons';

interface StatsPanelProps {
  round: ValidatedOreRound | undefined;
  treasuryMotherlode: number | null | undefined;
  isConnected: boolean;
  winnerSquare?: number | null;
  winnerRoundId?: number | null;
  minerDeployedThisRoundSol?: number;
}

export function StatsPanel({ round, treasuryMotherlode, isConnected, winnerSquare, winnerRoundId, minerDeployedThisRoundSol }: StatsPanelProps) {
  const [timeDisplay, setTimeDisplay] = useState<string>('–');

  useEffect(() => {
    const expiresAtUnix = round?.state.estimated_expires_at_unix;
    if (!expiresAtUnix) {
      setTimeDisplay('-');
      return;
    }

    const updateTimer = () => {
      const now = Math.floor(Date.now() / 1000);
      const remaining = expiresAtUnix - now;

      if (remaining <= 0) {
        setTimeDisplay('Round expired');
        return;
      }

      const minutes = Math.floor(remaining / 60);
      const seconds = remaining % 60;
      setTimeDisplay(`${String(minutes).padStart(2, '0')}:${String(seconds).padStart(2, '0')}`);
    };

    updateTimer();
    const interval = setInterval(updateTimer, 1000);
    return () => clearInterval(interval);
  }, [round?.state.estimated_expires_at_unix]);

  return (
    <div className="flex flex-col gap-6 h-full">
      <div className="bg-white dark:bg-stone-800 rounded-2xl p-8 shadow-sm dark:shadow-none dark:ring-1 dark:ring-stone-700">
        <div className="flex items-center gap-3 text-5xl font-bold text-stone-800 dark:text-stone-100">
          <OreIcon />
          <span>{treasuryMotherlode ?? '–'}</span>
        </div>
        <div className="text-base text-stone-500 dark:text-stone-400 mt-2">Motherlode</div>
      </div>

      <div className="bg-white dark:bg-stone-800 rounded-2xl p-8 shadow-sm dark:shadow-none dark:ring-1 dark:ring-stone-700">
        <div className={`font-semibold text-stone-800 dark:text-stone-100 tabular-nums ${timeDisplay.includes(':') ? 'text-5xl' : 'text-2xl'}`}>{timeDisplay}</div>
        <div className="text-base text-stone-500 dark:text-stone-400 mt-2">Time remaining</div>
      </div>

      <div className="grid grid-cols-2 gap-4">
        <div className="bg-white dark:bg-stone-800 rounded-2xl p-6 shadow-sm dark:shadow-none dark:ring-1 dark:ring-stone-700">
          <div className="flex items-center gap-2 text-2xl font-semibold text-stone-800 dark:text-stone-100">
            <SolanaIcon size={20} />
            <span>{round ? round.state.total_deployed.toFixed(4) : '0.0000'}</span>
          </div>
          <div className="text-base text-stone-500 dark:text-stone-400 mt-2">Total deployed</div>
        </div>
        <div className="bg-white dark:bg-stone-800 rounded-2xl p-6 shadow-sm dark:shadow-none dark:ring-1 dark:ring-stone-700">
          <div className="flex items-center gap-2 text-2xl font-semibold text-stone-800 dark:text-stone-100">
            <SolanaIcon size={20} />
            <span>{minerDeployedThisRoundSol != null ? minerDeployedThisRoundSol.toFixed(9).replace(/\.?0+$/, '') : '–'}</span>
          </div>
          <div className="text-base text-stone-500 dark:text-stone-400 mt-2">You deployed</div>
        </div>
      </div>

      <div className="bg-white dark:bg-stone-800 rounded-2xl p-4 shadow-sm dark:shadow-none dark:ring-1 dark:ring-stone-700">
        <div className="text-sm font-medium text-stone-700 dark:text-stone-300">Current / Last winner square</div>
        <div className="text-xl font-semibold text-amber-400 dark:text-amber-400 mt-1">
          {winnerSquare != null ? `#${winnerSquare + 1}` : '—'}
        </div>
        <div className="text-xs text-stone-500 dark:text-stone-400 mt-1">
          {winnerRoundId != null ? `Round ${winnerRoundId}` : 'Winner pending'}
        </div>
      </div>

      <div className="flex items-center gap-4 px-2 text-base text-stone-500 dark:text-stone-400">
        <span>Round {round?.id.round_id ?? '–'}</span>
        {round && (
          <>
            <span className="text-stone-300 dark:text-stone-600">·</span>
            <span>{round.state.total_miners} miners</span>
          </>
        )}
      </div>

      {!isConnected && (
        <div className="bg-amber-50 dark:bg-amber-900/30 text-amber-700 dark:text-amber-400 p-5 rounded-xl text-center">
          Connecting...
        </div>
      )}
    </div>
  );
}
