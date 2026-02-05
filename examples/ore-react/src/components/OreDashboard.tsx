import { useHyperstack } from 'hyperstack-react';
import {
  ORE_STREAM_STACK,
  type OreRound,
} from 'hyperstack-stacks/ore';
import { useState, useEffect } from 'react';

export function OreDashboard() {
  const { views, isConnected } = useHyperstack(ORE_STREAM_STACK, { url: "ws://localhost:8878" });
  const { data: latestRound } = views.OreRound.latest.useOne();
  const { data: treasuryData } = views.OreTreasury.list.useOne();

  return (
    <div style={styles.container}>
      <div style={styles.content}>
        {/* Left side - Grid */}
        <div style={styles.gridSection}>
          <BlockGrid round={latestRound} />
        </div>

        {/* Right side - Stats Panel */}
        <div style={styles.statsSection}>
          <StatsPanel
            round={latestRound}
            treasuryMotherlode={treasuryData?.state?.motherlode_ui}
            isConnected={isConnected}
          />
        </div>
      </div>

      {/* Connection indicator */}
      <div style={styles.connectionBadge}>
        <div style={{
          ...styles.connectionDot,
          backgroundColor: isConnected ? '#4ade80' : '#ef4444'
        }} />
        <span style={{
          ...styles.connectionText,
          color: isConnected ? '#fff' : '#fca5a5'
        }}>
          {isConnected ? 'CONNECTED' : 'DISCONNECTED'}
        </span>
      </div>
    </div>
  );
}

function BlockGrid({ round }: { round: OreRound | undefined }) {
  const deployedPerSquareUi = round?.state?.deployed_per_square_ui;
  const countPerSquare = round?.state?.count_per_square;

  const blocks = Array.from({ length: 25 }, (_, i) => ({
    id: i + 1,
    minerCount: countPerSquare?.[i] ?? 0,
    deployedUi: deployedPerSquareUi?.[i] ?? 0,
    isWinner: round?.results?.winning_square === i,
  }));

  return (
    <div style={styles.gridContainer}>
      {blocks.map((block) => (
        <div
          key={block.id}
          style={{
            ...styles.blockCard,
            ...(block.isWinner ? styles.winnerCard : {}),
          }}
        >
          <div style={styles.blockHeader}>
            <span style={styles.blockNumber}>#{block.id}</span>
            <div style={styles.minerInfo}>
              <span style={styles.minerCount}>{block.minerCount}</span>
              <MinerIcon />
            </div>
          </div>
          <div style={styles.blockDeployed}>
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
  round: OreRound | undefined;
  treasuryMotherlode: number | null | undefined;
  isConnected: boolean;
}) {
  const [timeRemaining, setTimeRemaining] = useState<string>('--:--');

  // Calculate time remaining
  useEffect(() => {
    if (!round?.state?.expires_at) {
      setTimeRemaining('--:--');
      return;
    }

    const updateTimer = () => {
      const now = Math.floor(Date.now() / 1000);
      const expiresAt = Number(round.state?.expires_at);
      const remaining = Math.max(0, expiresAt - now);

      const minutes = Math.floor(remaining / 60);
      const seconds = remaining % 60;
      setTimeRemaining(`${String(minutes).padStart(2, '0')}:${String(seconds).padStart(2, '0')}`);
    };

    updateTimer();
    const interval = setInterval(updateTimer, 1000);
    return () => clearInterval(interval);
  }, [round?.state?.expires_at]);

  const motherlodeValue = treasuryMotherlode ?? round?.state?.motherlode;

  return (
    <div style={styles.statsContainer}>
      {/* Top row - Motherlode and Time */}
      <div style={styles.statsRow}>
        <div style={styles.statCardHighlight}>
          <div style={styles.statValue}>
            <OreIcon />
            <span>{motherlodeValue}</span>
          </div>
          <div style={styles.statLabel}>Motherlode</div>
        </div>
        <div style={styles.statCard}>
          <div style={styles.statValueLarge}>{timeRemaining}</div>
          <div style={styles.statLabel}>Time remaining</div>
        </div>
      </div>

      {/* Second row - Deployed stats */}
      <div style={styles.statsRow}>
        <div style={styles.statCard}>
          <div style={styles.statValue}>
            <SolanaIcon />
            <span>{Number(round?.state?.total_deployed_ui ?? 0).toFixed(4)}</span>
          </div>
          <div style={styles.statLabel}>Total deployed</div>
        </div>
        <div style={styles.statCard}>
          <div style={styles.statValue}>
            <SolanaIcon />
            <span>0</span>
          </div>
          <div style={styles.statLabel}>You deployed</div>
        </div>
      </div>

      {/* Round info */}
      <div style={styles.roundInfo}>
        <div>
          <span style={styles.roundLabel}>Round #{round?.id?.round_id ?? '--'}</span>
          {round?.state?.total_miners != null && (
            <span style={{ ...styles.roundLabel, marginLeft: '16px' }}>
              {round.state.total_miners} miners
            </span>
          )}
        </div>
        {round?.results?.winning_square != null && (
          <span style={styles.winnerBadge}>
            Winner: Block #{(round.results.winning_square ?? 0) + 1}
          </span>
        )}
      </div>

      {/* Connection status */}
      {!isConnected && (
        <div style={styles.disconnectedBanner}>
          Waiting for connection...
        </div>
      )}
    </div>
  );
}

// Icons
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
      <circle cx="12" cy="12" r="10" stroke="#f59e0b" strokeWidth="2" fill="none" />
      <circle cx="12" cy="12" r="6" fill="#f59e0b" />
      <circle cx="12" cy="12" r="3" fill="#fbbf24" />
    </svg>
  );
}

// Styles
const styles: Record<string, React.CSSProperties> = {
  container: {
    minHeight: '100vh',
    width: '100%',
    backgroundColor: '#0a0a0a',
    padding: '24px',
    fontFamily: '-apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif',
    color: '#fff',
    position: 'relative',
  },
  content: {
    maxWidth: '1400px',
    margin: '0 auto',
    display: 'flex',
    gap: '32px',
    flexWrap: 'wrap' as const,
  },
  gridSection: {
    flex: '1 1 700px',
  },
  statsSection: {
    flex: '0 0 400px',
  },
  gridContainer: {
    display: 'grid',
    gridTemplateColumns: 'repeat(5, 1fr)',
    gap: '8px',
  },
  blockCard: {
    backgroundColor: '#1a1a1a',
    border: '1px solid #333',
    borderRadius: '8px',
    padding: '12px',
    display: 'flex',
    flexDirection: 'column' as const,
    gap: '24px',
    minHeight: '100px',
  },
  winnerCard: {
    border: '2px solid #fbbf24',
    boxShadow: '0 0 20px rgba(251, 191, 36, 0.3)',
  },
  blockHeader: {
    display: 'flex',
    justifyContent: 'space-between',
    alignItems: 'center',
  },
  blockNumber: {
    color: '#888',
    fontSize: '14px',
    fontWeight: 500,
  },
  minerInfo: {
    display: 'flex',
    alignItems: 'center',
    gap: '4px',
    color: '#888',
    fontSize: '14px',
  },
  minerCount: {
    color: '#ccc',
  },
  blockDeployed: {
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
    gap: '6px',
    fontSize: '16px',
    fontWeight: 600,
    color: '#fff',
  },
  statsContainer: {
    display: 'flex',
    flexDirection: 'column' as const,
    gap: '16px',
  },
  statsRow: {
    display: 'flex',
    gap: '12px',
  },
  statCard: {
    flex: 1,
    backgroundColor: '#1a1a1a',
    border: '1px solid #333',
    borderRadius: '12px',
    padding: '16px',
    display: 'flex',
    flexDirection: 'column' as const,
    alignItems: 'center',
    gap: '8px',
  },
  statCardHighlight: {
    flex: 1,
    backgroundColor: '#1a1a1a',
    border: '2px solid #f59e0b',
    borderRadius: '12px',
    padding: '16px',
    display: 'flex',
    flexDirection: 'column' as const,
    alignItems: 'center',
    gap: '8px',
  },
  statValue: {
    display: 'flex',
    alignItems: 'center',
    gap: '8px',
    fontSize: '24px',
    fontWeight: 700,
    color: '#fff',
  },
  statValueLarge: {
    fontSize: '28px',
    fontWeight: 700,
    color: '#fff',
  },
  statLabel: {
    fontSize: '12px',
    color: '#888',
    textTransform: 'uppercase' as const,
    letterSpacing: '0.5px',
  },
  roundInfo: {
    display: 'flex',
    justifyContent: 'space-between',
    alignItems: 'center',
    padding: '12px 16px',
    backgroundColor: '#1a1a1a',
    borderRadius: '8px',
    border: '1px solid #333',
  },
  roundLabel: {
    fontSize: '14px',
    color: '#888',
  },
  winnerBadge: {
    fontSize: '12px',
    color: '#fbbf24',
    backgroundColor: 'rgba(251, 191, 36, 0.1)',
    padding: '4px 8px',
    borderRadius: '4px',
  },
  disconnectedBanner: {
    backgroundColor: 'rgba(239, 68, 68, 0.1)',
    border: '1px solid rgba(239, 68, 68, 0.3)',
    color: '#fca5a5',
    padding: '12px',
    borderRadius: '8px',
    textAlign: 'center' as const,
    fontSize: '14px',
  },
  connectionBadge: {
    position: 'fixed' as const,
    top: '16px',
    right: '16px',
    display: 'flex',
    alignItems: 'center',
    gap: '8px',
    padding: '8px 16px',
    backgroundColor: 'rgba(26, 26, 26, 0.9)',
    borderRadius: '20px',
    border: '1px solid #333',
  },
  connectionDot: {
    width: '8px',
    height: '8px',
    borderRadius: '50%',
  },
  connectionText: {
    fontSize: '12px',
    fontWeight: 600,
    letterSpacing: '0.5px',
  },
};
