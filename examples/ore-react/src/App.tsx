import { OreDashboard } from './components/OreDashboard';
import { HyperstackProvider } from 'hyperstack-react';

export default function App() {
  return (
    <HyperstackProvider autoConnect={true}>
      <OreDashboard />
    </HyperstackProvider>
  );
}
