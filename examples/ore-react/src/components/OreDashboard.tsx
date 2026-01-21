import { useHyperstack, useConnectionState } from 'hyperstack-react';
import { OREROUND_STACK } from 'hyperstack-stacks/ore';

export function OreDashboard() {
  const stack = useHyperstack(OREROUND_STACK);
  const { data: latestRound } = stack.views.oreRound.latest.use();
  const connectionState = useConnectionState();
  const isConnected = connectionState === 'connected';

  return (
    <div className="min-h-screen w-full bg-gradient-to-b from-amber-900 to-yellow-600 px-4 pt-4 flex flex-col gap-4">
      {/* Navbar */}
      <nav className="w-full max-w-screen-xl px-4 sm:px-8 py-4 bg-black/20 backdrop-blur rounded-xl mx-auto">
        <div className="flex justify-between items-center">
          <div className="flex items-center gap-4">
            <h1 className="text-xl sm:text-2xl text-white font-bold">
              ORE Mining - Latest Round
            </h1>
            <p className="text-sm text-yellow-200 hidden sm:block">
              Real-time data via Hyperstack derived view
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
        {latestRound ? (
          <div className="bg-white rounded-3xl overflow-hidden shadow-xl">
            <div className="p-6 bg-gradient-to-r from-yellow-500 to-amber-500">
              <div className="flex items-center gap-2">
                <div className="w-3 h-3 rounded-full bg-white animate-pulse" />
                <span className="text-white font-bold text-sm uppercase tracking-wide">
                  Latest Round (Derived View)
                </span>
              </div>
              <div className="text-white text-3xl font-bold mt-2">
                Round #{latestRound.id?.roundId ?? '-'}
              </div>
            </div>
            <div className="p-6">
              <h3 className="text-lg font-medium text-gray-700 mb-4">Raw JSON Data</h3>
              <pre className="text-sm bg-gray-100 p-4 rounded-xl overflow-x-auto border border-gray-200 text-gray-800">
                {JSON.stringify(latestRound, null, 2)}
              </pre>
            </div>
          </div>
        ) : (
          <div className="text-center py-16">
            <div className="text-white">
              <h3 className="text-xl mb-2">Waiting for data...</h3>
              <p className="text-sm opacity-80">
                {isConnected ? 'Connected, waiting for latest round data' : 'Connect to see the latest round'}
              </p>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
