import { useState } from 'react';
import { useHyperstack, useConnectionState } from 'hyperstack-react';
import { OREROUND_STACK, OreRound } from 'hyperstack-stacks/ore';

function formatSol(lamports: number | undefined): string {
  if (lamports === undefined || lamports === null) return '-';
  return (lamports / 1_000_000_000).toFixed(6);
}

function formatNumber(value: number | undefined): string {
  if (value === undefined || value === null) return '-';
  return value.toString().replace(/\B(?=(\d{3})+(?!\d))/g, ',');
}

function shortenAddress(address: string | undefined): string {
  if (!address) return '-';
  return `${address.slice(0, 4)}...${address.slice(-4)}`;
}

function formatTimestamp(ts: number | undefined): string {
  if (!ts) return '-';
  // Convert unix timestamp (seconds) to readable date
  return new Date(ts * 1000).toLocaleString();
}

export function OreDashboard() {
  const stack = useHyperstack(OREROUND_STACK);
  const { data: rounds } = stack.views.oreRound.list.use();

  const connectionState = useConnectionState();

  const [expandedRounds, setExpandedRounds] = useState<Set<string>>(new Set());
  const [roundsPage, setRoundsPage] = useState(1);

  const ITEMS_PER_PAGE = 10;
  const isConnected = connectionState === 'connected';

  const roundsList = rounds ?? [];

  const RoundDetails = ({ round }: { round: OreRound }) => {
    const { id, metrics, results, state } = round;
    
    // Grid component for winning square (0-24)
    const winningSquare = results?.winningSquare;
    const squares = Array.from({ length: 25 }, (_, i) => i);

    return (
      <div className="p-6 bg-amber-50">
        <h4 className="text-lg font-medium text-amber-900 mb-6">Round Details</h4>

        <div className="grid grid-cols-1 md:grid-cols-2 gap-8">
          {/* Left Column: Stats */}
          <div className="space-y-6">
            {/* Round State */}
            <div>
              <h5 className="text-sm font-medium text-amber-800 mb-3">Round State</h5>
              <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
                {state?.motherlode !== undefined && (
                  <div className="bg-white p-4 rounded-lg border border-amber-200">
                    <div className="text-sm text-amber-600">Motherlode Jackpot</div>
                    <div className="text-lg font-medium text-amber-900">
                      {formatSol(state.motherlode)} SOL
                    </div>
                  </div>
                )}
                {state?.totalDeployed !== undefined && (
                  <div className="bg-white p-4 rounded-lg border border-amber-200">
                    <div className="text-sm text-amber-600">Total Deployed</div>
                    <div className="text-lg font-medium text-amber-900">
                      {formatSol(state.totalDeployed)} SOL
                    </div>
                  </div>
                )}
                {state?.totalVaulted !== undefined && (
                  <div className="bg-white p-4 rounded-lg border border-amber-200">
                    <div className="text-sm text-amber-600">Total Vaulted</div>
                    <div className="text-lg font-medium text-amber-900">
                      {formatSol(state.totalVaulted)} SOL
                    </div>
                  </div>
                )}
                {state?.totalWinnings !== undefined && (
                  <div className="bg-white p-4 rounded-lg border border-amber-200">
                    <div className="text-sm text-amber-600">Total Winnings</div>
                    <div className="text-lg font-medium text-amber-900">
                      {formatSol(state.totalWinnings)} SOL
                    </div>
                  </div>
                )}
              </div>
            </div>

            {/* Metrics */}
            <div>
              <h5 className="text-sm font-medium text-amber-800 mb-3">Metrics</h5>
              <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
                {metrics?.deployCount !== undefined && (
                  <div className="bg-white p-4 rounded-lg border border-amber-200">
                    <div className="text-sm text-amber-600">Deploy Count</div>
                    <div className="text-lg font-medium text-gray-900">
                      {formatNumber(metrics.deployCount)}
                    </div>
                  </div>
                )}
                {metrics?.totalDeployedSol !== undefined && (
                  <div className="bg-white p-4 rounded-lg border border-amber-200">
                    <div className="text-sm text-amber-600">Total Deployed SOL</div>
                    <div className="text-lg font-medium text-gray-900">
                      {formatSol(metrics.totalDeployedSol)}
                    </div>
                  </div>
                )}
                {metrics?.checkpointCount !== undefined && (
                  <div className="bg-white p-4 rounded-lg border border-amber-200">
                    <div className="text-sm text-amber-600">Checkpoint Count</div>
                    <div className="text-lg font-medium text-gray-900">
                      {formatNumber(metrics.checkpointCount)}
                    </div>
                  </div>
                )}
              </div>
            </div>
            
            {/* Result Details */}
            <div>
              <h5 className="text-sm font-medium text-amber-800 mb-3">Result</h5>
              <div className="bg-white p-4 rounded-lg border border-amber-200 space-y-2">
                <div className="flex justify-between">
                  <span className="text-sm text-amber-600">Top Miner</span>
                  <span className="text-sm font-medium">{shortenAddress(results?.topMiner || undefined)}</span>
                </div>
                <div className="flex justify-between">
                  <span className="text-sm text-amber-600">Miner Reward</span>
                  <span className="text-sm font-medium">{formatSol(results?.topMinerReward || undefined)} SOL</span>
                </div>
                <div className="flex justify-between">
                  <span className="text-sm text-amber-600">Hit Motherlode?</span>
                  <span className={`text-sm font-medium ${results?.didHitMotherlode ? 'text-green-600' : 'text-gray-600'}`}>
                    {results?.didHitMotherlode ? 'YES' : 'NO'}
                  </span>
                </div>
              </div>
            </div>
          </div>

          {/* Right Column: Grid Visualization */}
          <div>
            <h5 className="text-sm font-medium text-amber-800 mb-3">Winning Square Grid</h5>
            <div className="bg-white p-6 rounded-lg border border-amber-200 flex flex-col items-center justify-center">
              <div className="grid grid-cols-5 gap-2 w-full max-w-[300px] aspect-square">
                {squares.map((i) => {
                  const isWinner = i === winningSquare;
                  return (
                    <div
                      key={i}
                      className={`
                        rounded flex items-center justify-center text-xs font-bold transition-all
                        ${isWinner 
                          ? 'bg-amber-500 text-white shadow-lg scale-110 ring-2 ring-amber-300 z-10' 
                          : 'bg-amber-100 text-amber-300'}
                      `}
                    >
                      {i}
                    </div>
                  );
                })}
              </div>
              <div className="mt-4 text-center text-sm text-amber-700">
                {winningSquare !== undefined && winningSquare !== null 
                  ? `Winning Square: #${winningSquare}` 
                  : 'Round in progress...'}
              </div>
            </div>

            {/* Raw JSON */}
            <div className="mt-6">
              <div className="text-sm text-amber-600 mb-2">Raw JSON Data</div>
              <pre className="text-xs bg-gray-100 p-3 rounded overflow-x-auto max-h-48 border border-gray-200">
                {JSON.stringify(round, null, 2)}
              </pre>
            </div>
          </div>
        </div>
      </div>
    );
  };

  return (
    <div className="min-h-screen w-full bg-gradient-to-b from-amber-900 to-yellow-600 px-4 pt-4 flex flex-col gap-4">
      {/* Navbar */}
      <nav className="w-full max-w-screen-xl px-4 sm:px-8 py-4 bg-black/20 backdrop-blur rounded-xl mx-auto">
        <div className="flex justify-between items-center">
          <div className="flex items-center gap-4">
            <h1 className="text-xl sm:text-2xl text-white font-bold">
              ORE Mining Rounds
            </h1>
            <p className="text-sm text-yellow-200 hidden sm:block">
              Real-time mining data via Hyperstack
            </p>
          </div>

          <div
            className="flex items-center gap-3 px-4 py-2 rounded-xl bg-white/10 cursor-pointer hover:bg-white/20 transition-all"
            onClick={isConnected ? () => stack.runtime.connection.disconnect() : () => stack.runtime.connection.connect()}
          >
            <div className="relative">
              <div className={`w-3 h-3 rounded-full ${isConnected ? 'bg-green-400' : 'bg-red-500'}`} />
              {isConnected && (
                <div className="absolute inset-0 w-3 h-3 rounded-full bg-green-400 animate-ping" />
              )}
            </div>
            <span className={`font-medium text-sm ${isConnected ? 'text-white' : 'text-red-300'}`}>
              {isConnected ? 'CONNECTED' : 'DISCONNECTED'}
            </span>
          </div>
        </div>
      </nav>

      <div className="w-full max-w-screen-xl mx-auto pb-16 pt-4 flex-1">
        <div className="flex flex-col items-stretch gap-6 h-full">
          <div className="w-full bg-amber-600/80 backdrop-blur rounded-3xl overflow-hidden flex flex-col shadow-xl">
            <div className="p-6 flex-1 flex flex-col">
              <div className="px-3 py-2 bg-white rounded-md inline-block mb-4 w-fit shadow-sm">
                <h2 className="text-amber-900 text-lg font-medium">
                  Active Rounds ({roundsList.length})
                </h2>
              </div>

              {roundsList.length === 0 ? (
                <div className="text-center py-16">
                  <div className="text-white">
                    <h3 className="text-xl mb-2">No Rounds Found</h3>
                    <p className="text-sm opacity-80">
                      ORE mining rounds will appear here from Solana
                    </p>
                  </div>
                </div>
              ) : (
                <div className="flex-1 flex flex-col">
                  <div className="space-y-3 flex-1 overflow-y-auto">
                    {roundsList
                      .slice((roundsPage - 1) * ITEMS_PER_PAGE, roundsPage * ITEMS_PER_PAGE)
                      .map((round: OreRound, index: number) => {
                        const roundKey = round.id?.roundAddress || `round-${index}`;
                        const isExpanded = expandedRounds.has(roundKey);
                        const isExpired = round.state?.expiresAt ? (round.state.expiresAt * 1000) < Date.now() : false;

                        return (
                          <div
                            key={roundKey}
                            className="bg-white rounded-xl overflow-hidden transition-all hover:scale-[1.005] shadow-sm"
                          >
                            <div
                              className="p-4 cursor-pointer hover:bg-amber-50 transition-colors"
                              onClick={() => {
                                setExpandedRounds(prev => {
                                  const newSet = new Set(prev);
                                  if (newSet.has(roundKey)) {
                                    newSet.delete(roundKey);
                                  } else {
                                    newSet.add(roundKey);
                                  }
                                  return newSet;
                                });
                              }}
                            >
                              <div className="flex justify-between items-start gap-4">
                                <div>
                                  <div className="text-lg text-gray-800 font-medium flex items-center gap-2">
                                    Round #{round.id?.roundId ?? '-'}
                                    <span className="text-sm font-normal text-gray-400">
                                      ({shortenAddress(round.id?.roundAddress || undefined)})
                                    </span>
                                  </div>
                                  <div className="text-sm text-gray-600 mt-1">
                                    Ends: {formatTimestamp(round.state?.expiresAt || undefined)}
                                    {round.state?.motherlode !== undefined && (
                                      <span className="ml-2 font-medium text-amber-600">
                                        • Jackpot: {formatSol(round.state.motherlode)} SOL
                                      </span>
                                    )}
                                  </div>
                                  <div className="text-xs text-gray-500 mt-1">
                                    {round.metrics?.deployCount !== undefined && (
                                      <>{round.metrics.deployCount} deploys</>
                                    )}
                                    {round.metrics?.totalDeployedSol !== undefined && (
                                      <> • {formatSol(round.metrics.totalDeployedSol)} SOL deployed</>
                                    )}
                                  </div>
                                </div>
                                <div className="flex items-center gap-2">
                                  {isExpired && (
                                    <span className="px-2 py-1 rounded text-xs font-medium bg-gray-100 text-gray-600">
                                      Ended
                                    </span>
                                  )}
                                  {round.results?.didHitMotherlode && (
                                    <span className="px-2 py-1 rounded text-xs font-medium bg-yellow-100 text-yellow-700 border border-yellow-200">
                                      JACKPOT!
                                    </span>
                                  )}
                                  <div className={`transform transition-transform ${isExpanded ? 'rotate-90' : ''}`}>
                                    <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor" className="text-gray-500">
                                      <path d="M6 4l4 4-4 4" stroke="currentColor" strokeWidth="2" fill="none" />
                                    </svg>
                                  </div>
                                </div>
                              </div>
                            </div>

                            {isExpanded && (
                              <div className="border-t border-amber-100">
                                <RoundDetails round={round} />
                              </div>
                            )}
                          </div>
                        );
                      })}
                  </div>

                  {roundsList.length > ITEMS_PER_PAGE && (
                    <div className="flex justify-center items-center gap-2 mt-4 pt-4 border-t border-white/30">
                      <button
                        onClick={() => setRoundsPage(p => Math.max(1, p - 1))}
                        disabled={roundsPage === 1}
                        className="px-3 py-1 rounded-md text-sm bg-white text-amber-600 disabled:opacity-50 disabled:cursor-not-allowed hover:bg-gray-100 transition-colors"
                      >
                        Previous
                      </button>
                      <span className="px-3 py-1 text-sm text-white font-medium">
                        Page {roundsPage} of {Math.ceil(roundsList.length / ITEMS_PER_PAGE)}
                      </span>
                      <button
                        onClick={() => setRoundsPage(p => Math.min(Math.ceil(roundsList.length / ITEMS_PER_PAGE), p + 1))}
                        disabled={roundsPage >= Math.ceil(roundsList.length / ITEMS_PER_PAGE)}
                        className="px-3 py-1 rounded-md text-sm bg-white text-amber-600 disabled:opacity-50 disabled:cursor-not-allowed hover:bg-gray-100 transition-colors"
                      >
                        Next
                      </button>
                    </div>
                  )}
                </div>
              )}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
