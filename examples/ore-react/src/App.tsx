import { OreDashboard } from './components/OreDashboard';
import { HyperstackProvider } from 'hyperstack-react';

const websocketUrl = import.meta.env.VITE_HYPERSTACK_WS_URL;

export default function App() {
  return (
    <HyperstackProvider
      websocketUrl={websocketUrl}
      autoConnect={true}
    >
      <OreDashboard />
    </HyperstackProvider>
  );
}
