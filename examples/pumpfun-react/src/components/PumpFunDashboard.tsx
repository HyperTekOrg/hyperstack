import { useState } from 'react';
import { useHyperstack, useConnectionState } from 'hyperstack-react';
import { PUMPFUNTOKEN_STACK } from 'hyperstack-stacks/pumpfun';
import { TokenBuyButton } from './TokenBuyButton';

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

export function PumpFunDashboard() {
  const stack = useHyperstack(PUMPFUNTOKEN_STACK);
  const { data: tokens } = stack.views.PumpfunToken.list.use();

  const connectionState = useConnectionState();

  const [expandedTokens, setExpandedTokens] = useState<Set<string>>(new Set());
  const [tokensPage, setTokensPage] = useState(1);

  const ITEMS_PER_PAGE = 10;
  const isConnected = connectionState === 'connected';

  const tokensList = tokens ?? [];

  const TokenDetails = ({ token }: { token: any }) => {
    const info = token.info;
    const reserves = token.reserves;
    const trading = token.trading;

    return (
      <div className="p-6 bg-gray-50">
        <h4 className="text-lg font-medium text-gray-800 mb-6">Token Details</h4>

        {/* Token Info */}
        {(info?.name || info?.symbol || info?.uri || info?.is_complete !== undefined) && (
          <div className="mb-6">
            <h5 className="text-sm font-medium text-gray-700 mb-3">Token Info</h5>
            <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
              {info?.name && (
                <div className="bg-white p-4 rounded-lg border">
                  <div className="text-sm text-gray-600">Name</div>
                  <div className="text-lg font-medium text-gray-900">{info.name}</div>
                </div>
              )}
              {info?.symbol && (
                <div className="bg-white p-4 rounded-lg border">
                  <div className="text-sm text-gray-600">Symbol</div>
                  <div className="text-lg font-medium text-blue-600">${info.symbol}</div>
                </div>
              )}
              {info?.uri && (
                <div className="bg-white p-4 rounded-lg border">
                  <div className="text-sm text-gray-600">Metadata URI</div>
                  <a
                    href={info.uri}
                    target="_blank"
                    rel="noopener noreferrer"
                    className="text-sm font-medium text-blue-600 hover:underline break-all"
                  >
                    {info.uri.length > 50 ? `${info.uri.slice(0, 50)}...` : info.uri}
                  </a>
                </div>
              )}
              {info?.is_complete !== undefined && (
                <div className="bg-white p-4 rounded-lg border">
                  <div className="text-sm text-gray-600">Status</div>
                  <div className={`text-lg font-medium ${info.is_complete ? 'text-green-600' : 'text-yellow-600'}`}>
                    {info.is_complete ? 'Complete' : 'In Progress'}
                  </div>
                </div>
              )}
            </div>
          </div>
        )}

        {/* Reserves Info */}
        <div className="mb-6">
          <h5 className="text-sm font-medium text-gray-700 mb-3">Reserves</h5>
          <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4">
            {reserves?.virtualSolReserves !== undefined && (
              <div className="bg-white p-4 rounded-lg border">
                <div className="text-sm text-gray-600">Virtual SOL Reserves</div>
                <div className="text-lg font-medium text-gray-900">
                  {formatSol(reserves.virtualSolReserves)} SOL
                </div>
              </div>
            )}
            {reserves?.realSolReserves !== undefined && (
              <div className="bg-white p-4 rounded-lg border">
                <div className="text-sm text-gray-600">Real SOL Reserves</div>
                <div className="text-lg font-medium text-green-600">
                  {formatSol(reserves.realSolReserves)} SOL
                </div>
              </div>
            )}
            {reserves?.tokenTotalSupply !== undefined && (
              <div className="bg-white p-4 rounded-lg border">
                <div className="text-sm text-gray-600">Total Supply</div>
                <div className="text-lg font-medium text-blue-600">
                  {formatNumber(reserves.tokenTotalSupply)}
                </div>
              </div>
            )}
          </div>
        </div>

        {/* Trading Stats */}
        <div className="mb-6">
          <h5 className="text-sm font-medium text-gray-700 mb-3">Trading Statistics</h5>
          <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4">
            {trading?.totalVolume !== undefined && (
              <div className="bg-white p-4 rounded-lg border">
                <div className="text-sm text-gray-600">Total Volume</div>
                <div className="text-lg font-medium text-gray-900">
                  {formatNumber(trading.totalVolume)}
                </div>
              </div>
            )}
            {trading?.total_trades !== undefined && (
              <div className="bg-white p-4 rounded-lg border">
                <div className="text-sm text-gray-600">Total Trades</div>
                <div className="text-lg font-medium text-purple-600">
                  {formatNumber(trading.total_trades)}
                </div>
              </div>
            )}
            {trading?.unique_traders !== undefined && (
              <div className="bg-white p-4 rounded-lg border">
                <div className="text-sm text-gray-600">Unique Traders</div>
                <div className="text-lg font-medium text-blue-600">
                  {formatNumber(trading.unique_traders)}
                </div>
              </div>
            )}
            {trading?.buyCount !== undefined && (
              <div className="bg-white p-4 rounded-lg border">
                <div className="text-sm text-gray-600">Buy Count</div>
                <div className="text-lg font-medium text-green-600">
                  {formatNumber(trading.buyCount)}
                </div>
              </div>
            )}
            {trading?.sellCount !== undefined && (
              <div className="bg-white p-4 rounded-lg border">
                <div className="text-sm text-gray-600">Sell Count</div>
                <div className="text-lg font-medium text-red-600">
                  {formatNumber(trading.sellCount)}
                </div>
              </div>
            )}
            {trading?.last_trade_price !== undefined && (
              <div className="bg-white p-4 rounded-lg border">
                <div className="text-sm text-gray-600">Last Trade Price</div>
                <div className="text-lg font-medium text-gray-900">
                  {trading.last_trade_price.toFixed(10)} SOL
                </div>
              </div>
            )}
          </div>
        </div>

        {/* Buy Token Section */}
        {(token.id?.mint && token.id?.bonding_curve) && (
          <div className="mb-6">
            <h5 className="text-sm font-medium text-gray-700 mb-3">Trade Token</h5>
            <TokenBuyButton
              mint={token.id.mint}
              bondingCurve={token.id.bonding_curve}
              tokenSymbol={token.info?.symbol}
            />
          </div>
        )}

        {/* Raw JSON */}
        <div className="bg-white p-4 rounded-lg border">
          <div className="text-sm text-gray-600 mb-2">Raw JSON Data</div>
          <pre className="text-xs bg-gray-100 p-3 rounded overflow-x-auto max-h-64">
            {JSON.stringify(token, null, 2)}
          </pre>
        </div>
      </div>
    );
  };

  return (
    <div className="min-h-screen w-full bg-gradient-to-b from-purple-900 to-pink-700 px-4 pt-4 flex flex-col gap-4">
      {/* Navbar */}
      <nav className="w-full max-w-screen-xl px-4 sm:px-8 py-4 bg-black/20 backdrop-blur rounded-xl mx-auto">
        <div className="flex justify-between items-center">
          <div className="flex items-center gap-4">
            <h1 className="text-xl sm:text-2xl text-white font-bold">
              PumpFun Token Stream
            </h1>
            <p className="text-sm text-gray-300 hidden sm:block">
              Real-time Solana token data via Hyperstack
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
          <div className="w-full bg-pink-600/80 backdrop-blur rounded-3xl overflow-hidden flex flex-col">
            <div className="p-6 flex-1 flex flex-col">
              <div className="px-3 py-2 bg-white rounded-md inline-block mb-4 w-fit">
                <h2 className="text-gray-800 text-lg font-medium">
                  PumpFun Tokens ({tokensList.length})
                </h2>
              </div>

              {tokensList.length === 0 ? (
                <div className="text-center py-16">
                  <div className="text-white">
                    <h3 className="text-xl mb-2">No Tokens</h3>
                    <p className="text-sm opacity-80">
                      PumpFun tokens will appear here from Solana
                    </p>
                  </div>
                </div>
              ) : (
                <div className="flex-1 flex flex-col">
                  <div className="space-y-3 flex-1 overflow-y-auto">
                    {tokensList
                      .slice((tokensPage - 1) * ITEMS_PER_PAGE, tokensPage * ITEMS_PER_PAGE)
                      .map((token: any, index: number) => {
                        const tokenKey = token.bonding_curve_snapshot?.creator || token.id?.bonding_curve || `token-${index}`;
                        const isExpanded = expandedTokens.has(tokenKey);
                        const isComplete = token.info?.is_complete || token.bonding_curve_snapshot?.complete;

                        return (
                          <div
                            key={tokenKey}
                            className="bg-white rounded-xl overflow-hidden transition-all hover:scale-[1.01]"
                          >
                            <div
                              className="p-4 cursor-pointer hover:bg-gray-50 transition-colors"
                              onClick={() => {
                                setExpandedTokens(prev => {
                                  const newSet = new Set(prev);
                                  if (newSet.has(tokenKey)) {
                                    newSet.delete(tokenKey);
                                  } else {
                                    newSet.add(tokenKey);
                                  }
                                  return newSet;
                                });
                              }}
                            >
                              <div className="flex justify-between items-start gap-4">
                                <div>
                                  <div className="text-lg text-gray-800 font-medium flex items-center gap-2">
                                    {token.info?.name || `Token #${index + 1}`}
                                    {token.info?.symbol && (
                                      <span className="text-sm font-normal text-gray-500">
                                        ${token.info.symbol}
                                      </span>
                                    )}
                                  </div>
                                  <div className="text-sm text-gray-600 mt-1">
                                    {token.id?.mint && (
                                      <>Mint: {shortenAddress(token.id.mint)} | </>
                                    )}
                                    Bonding Curve: {shortenAddress(tokenKey)}
                                    {token.trading?.last_trade_price !== undefined && (
                                      <> | Price: {token.trading.last_trade_price.toFixed(10)} SOL</>
                                    )}
                                  </div>
                                  <div className="text-xs text-gray-500 mt-1">
                                    {token.trading?.total_trades !== undefined && (
                                      <>{token.trading.total_trades} trades</>
                                    )}
                                    {token.trading?.unique_traders !== undefined && (
                                      <> • {token.trading.unique_traders} traders</>
                                    )}
                                    {token.info?.uri && (
                                      <> • <a href={token.info.uri} target="_blank" rel="noopener noreferrer" className="text-blue-600 hover:underline" onClick={e => e.stopPropagation()}>Metadata</a></>
                                    )}
                                  </div>
                                </div>
                                <div className="flex items-center gap-2">
                                  {isComplete && (
                                    <span className="px-2 py-1 rounded text-xs font-medium bg-green-100 text-green-700">
                                      Complete
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
                              <div className="border-t border-gray-200">
                                <TokenDetails token={token} />
                              </div>
                            )}
                          </div>
                        );
                      })}
                  </div>

                  {tokensList.length > ITEMS_PER_PAGE && (
                    <div className="flex justify-center items-center gap-2 mt-4 pt-4 border-t border-white/30">
                      <button
                        onClick={() => setTokensPage(p => Math.max(1, p - 1))}
                        disabled={tokensPage === 1}
                        className="px-3 py-1 rounded-md text-sm bg-white text-pink-600 disabled:opacity-50 disabled:cursor-not-allowed hover:bg-gray-100 transition-colors"
                      >
                        Previous
                      </button>
                      <span className="px-3 py-1 text-sm text-white">
                        Page {tokensPage} of {Math.ceil(tokensList.length / ITEMS_PER_PAGE)}
                      </span>
                      <button
                        onClick={() => setTokensPage(p => Math.min(Math.ceil(tokensList.length / ITEMS_PER_PAGE), p + 1))}
                        disabled={tokensPage >= Math.ceil(tokensList.length / ITEMS_PER_PAGE)}
                        className="px-3 py-1 rounded-md text-sm bg-white text-pink-600 disabled:opacity-50 disabled:cursor-not-allowed hover:bg-gray-100 transition-colors"
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
