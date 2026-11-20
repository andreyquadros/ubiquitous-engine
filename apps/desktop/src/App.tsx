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
import { FocusPage } from './pages/Focus';
import { InterventionPage } from './pages/Intervention';
import { UbiqxNofapCard, UbiqxNofapPage } from './nofap/ubiqx';

export default function App() {
  return (
    <HashRouter>
      <Routes>
        {/* The floating intervention panel (second Tauri window): no gate, no shell. */}
        <Route path="/intervention" element={<InterventionPage />} />
        {/* The module's floating card window (second Tauri window, label `shield-card`): no gate, no shell. */}
        <Route path="/nofap-card" element={<UbiqxNofapCard />} />
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
            <Route path="/focus" element={<FocusPage />} />
            <Route path="/nofap" element={<UbiqxNofapPage />} />
            <Route path="*" element={<Dashboard />} />
          </Route>
        </Route>
      </Routes>
    </HashRouter>
  );
}
