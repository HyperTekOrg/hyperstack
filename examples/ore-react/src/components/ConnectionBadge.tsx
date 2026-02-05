interface ConnectionBadgeProps {
  isConnected: boolean;
}

export function ConnectionBadge({ isConnected }: ConnectionBadgeProps) {
  return (
    <div className="fixed top-4 right-4 flex items-center gap-2 px-4 py-2 bg-slate-900/90 backdrop-blur-sm rounded-full border border-slate-700/50">
      <div className={`w-2 h-2 rounded-full ${isConnected ? 'bg-emerald-400 shadow-[0_0_8px_rgba(52,211,153,0.5)]' : 'bg-red-500'}`} />
      <span className={`text-xs font-semibold tracking-wide ${isConnected ? 'text-white' : 'text-red-300'}`}>
        {isConnected ? 'CONNECTED' : 'DISCONNECTED'}
      </span>
    </div>
  );
}
