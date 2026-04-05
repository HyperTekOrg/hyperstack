import { OreDashboard } from './components';
import { HyperstackProvider } from 'hyperstack-react';
import { ThemeProvider } from './hooks/useTheme';

// Use your own publishable key in production
const PUBLISHABLE_KEY = 'hspk_alt8MN3BmJebxARE3IlOnnaAEibCrqqXfdG5VoGW';

export default function App() {
  return (
    <ThemeProvider>
      <HyperstackProvider
        autoConnect={true}
        auth={{
          publishableKey: PUBLISHABLE_KEY,
        }}
      >
        <OreDashboard />
      </HyperstackProvider>
    </ThemeProvider>
  );
}
