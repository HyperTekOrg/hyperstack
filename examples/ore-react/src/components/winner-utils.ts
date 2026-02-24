import type { OreRound } from 'hyperstack-stacks/ore';
import type { ValidatedOreRound } from '../schemas/ore-round-validated';

export type WinnerInfo = {
  winnerSquare: number | null;
  winnerRoundId: number | null;
};

export function getCurrentRoundWinner(round: ValidatedOreRound | undefined, nowUnix: number): WinnerInfo {
  const roundId = round?.id?.round_id;
  const finished =
    typeof round?.state?.estimated_expires_at_unix === 'number' &&
    round.state.estimated_expires_at_unix <= nowUnix;

  if (!finished || typeof round?.results?.winning_square !== 'number') {
    return { winnerSquare: null, winnerRoundId: null };
  }

  return {
    winnerSquare: round.results.winning_square,
    winnerRoundId: roundId ?? null,
  };
}

export function getLastCompletedWinner(recentRounds: OreRound[] | undefined, currentRoundId: number | null): WinnerInfo {
  const bestRound = recentRounds?.reduce<OreRound | undefined>((best, round) => {
    const roundId = round?.id?.round_id;
    const winner = round?.results?.winning_square;

    if (typeof roundId !== 'number' || typeof winner !== 'number') {
      return best;
    }

    if (currentRoundId != null && roundId >= currentRoundId) {
      return best;
    }

    if (!best) {
      return round;
    }

    const bestRoundId = best.id?.round_id;
    if (typeof bestRoundId !== 'number' || roundId > bestRoundId) {
      return round;
    }

    return best;
  }, undefined);

  return {
    winnerSquare:
      typeof bestRound?.results?.winning_square === 'number'
        ? bestRound.results.winning_square
        : null,
    winnerRoundId: bestRound?.id?.round_id ?? null,
  };
}

export function getStatsWinner(
  currentRound: ValidatedOreRound | undefined,
  recentRounds: OreRound[] | undefined,
  nowUnix: number
): WinnerInfo {
  const currentRoundId = currentRound?.id?.round_id ?? null;
  const currentWinner = getCurrentRoundWinner(currentRound, nowUnix);

  if (currentWinner.winnerSquare != null) {
    return currentWinner;
  }

  return getLastCompletedWinner(recentRounds, currentRoundId);
}
