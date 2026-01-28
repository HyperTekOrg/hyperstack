import { OreDashboard } from './components/OreDashboard';
import { HyperstackProvider } from 'hyperstack-react';

export default function App() {
  return (
    <HyperstackProvider
      websocketUrl="wss://ore.stack.usehyperstack.com"
      autoConnect={true}
    >
      <OreDashboard />
    </HyperstackProvider>
  );
}
