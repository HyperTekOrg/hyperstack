import { useState, useEffect } from 'react';
import type { ValidatedOreRound } from '../schemas/ore-round-validated';
import { OreIcon, SolanaIcon } from './icons';

interface StatsPanelProps {
  round: ValidatedOreRound | undefined;
  treasuryMotherlode: number | null | undefined;
  isConnected: boolean;
}

export function StatsPanel({ round, treasuryMotherlode, isConnected }: StatsPanelProps) {
  const [timeRemaining, setTimeRemaining] = useState<string>('00:00');

  useEffect(() => {
    const expiresAtUnix = round?.state.estimated_expires_at_unix;
    if (!expiresAtUnix) {
      setTimeRemaining('00:00');
      return;
    }

    const updateTimer = () => {
      const now = Math.floor(Date.now() / 1000);
      const remaining = Math.max(0, expiresAtUnix - now);

      if (remaining > 300) {
        setTimeRemaining('00:00');
        return;
      }

      const minutes = Math.floor(remaining / 60);
      const seconds = remaining % 60;
      setTimeRemaining(`${String(minutes).padStart(2, '0')}:${String(seconds).padStart(2, '0')}`);
    };

    updateTimer();
    const interval = setInterval(updateTimer, 1000);
    return () => clearInterval(interval);
  }, [round?.state.estimated_expires_at_unix]);

  return (
    <div className="flex flex-col gap-4">
      <div className="bg-white dark:bg-stone-800 rounded-2xl p-6 shadow-sm dark:shadow-none dark:ring-1 dark:ring-stone-700">
        <div className="flex items-center gap-2 text-3xl font-bold text-stone-800 dark:text-stone-100">
          <OreIcon />
          <span>{treasuryMotherlode ?? '–'}</span>
        </div>
        <div className="text-sm text-stone-500 dark:text-stone-400 mt-1">Motherlode</div>
      </div>

      <div className="bg-white dark:bg-stone-800 rounded-2xl p-6 shadow-sm dark:shadow-none dark:ring-1 dark:ring-stone-700">
        <div className="text-3xl font-semibold text-stone-800 dark:text-stone-100 tabular-nums">{timeRemaining}</div>
        <div className="text-sm text-stone-500 dark:text-stone-400 mt-1">Time remaining</div>
      </div>

      <div className="grid grid-cols-2 gap-3">
        <div className="bg-white dark:bg-stone-800 rounded-2xl p-5 shadow-sm dark:shadow-none dark:ring-1 dark:ring-stone-700">
          <div className="flex items-center gap-1.5 text-xl font-semibold text-stone-800 dark:text-stone-100">
            <SolanaIcon size={16} />
            <span>{round ? round.state.total_deployed_ui.toFixed(4) : '0.0000'}</span>
          </div>
          <div className="text-sm text-stone-500 dark:text-stone-400 mt-1">Total deployed</div>
        </div>
        <div className="bg-white dark:bg-stone-800 rounded-2xl p-5 shadow-sm dark:shadow-none dark:ring-1 dark:ring-stone-700">
          <div className="flex items-center gap-1.5 text-xl font-semibold text-stone-800 dark:text-stone-100">
            <SolanaIcon size={16} />
            <span>0</span>
          </div>
          <div className="text-sm text-stone-500 dark:text-stone-400 mt-1">You deployed</div>
        </div>
      </div>

      <div className="flex items-center gap-3 px-1 text-sm text-stone-500 dark:text-stone-400">
        <span>Round {round?.id.round_id ?? '–'}</span>
        {round && (
          <>
            <span className="text-stone-300 dark:text-stone-600">·</span>
            <span>{round.state.total_miners} miners</span>
          </>
        )}
      </div>

      {!isConnected && (
        <div className="bg-amber-50 dark:bg-amber-900/30 text-amber-700 dark:text-amber-400 p-4 rounded-xl text-center text-sm">
          Connecting...
        </div>
      )}
    </div>
  );
}
