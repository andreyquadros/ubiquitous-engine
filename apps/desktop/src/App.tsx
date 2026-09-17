import { HashRouter, Route, Routes } from 'react-router-dom';
import { AppShell, Gate } from './components/layout/AppShell';
import { Dashboard } from './pages/Dashboard';
import { Timeline } from './pages/Timeline';
import { Review } from './pages/Review';
import { Reports } from './pages/Reports';
import { Categories } from './pages/Categories';
import { Insights } from './pages/Insights';
import { SettingsPage } from './pages/Settings';
import { Onboarding } from './pages/Onboarding';

export default function App() {
  return (
    <HashRouter>
      <Routes>
        <Route element={<Gate />}>
          <Route path="/onboarding" element={<Onboarding />} />
          <Route element={<AppShell />}>
            <Route index element={<Dashboard />} />
            <Route path="/timeline" element={<Timeline />} />
            <Route path="/review" element={<Review />} />
            <Route path="/reports" element={<Reports />} />
            <Route path="/categories" element={<Categories />} />
            <Route path="/insights" element={<Insights />} />
            <Route path="/settings" element={<SettingsPage />} />
            <Route path="*" element={<Dashboard />} />
          </Route>
        </Route>
      </Routes>
    </HashRouter>
  );
}
