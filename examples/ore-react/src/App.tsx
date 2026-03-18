import { OreDashboard } from './components';
import { HyperstackProvider } from 'hyperstack-react';
import { ThemeProvider } from './hooks/useTheme';

export default function App() {
  return (
    <ThemeProvider>
      <HyperstackProvider autoConnect={true}>
        <OreDashboard />
      </HyperstackProvider>
    </ThemeProvider>
  );
}
