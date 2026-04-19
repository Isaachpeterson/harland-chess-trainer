import { HashRouter, Routes, Route, NavLink } from "react-router-dom";
import { SettingsPage } from "./pages/SettingsPage";
import { SyncPage } from "./pages/SyncPage";
import "./App.css";

function App() {
  return (
    <HashRouter>
      <div className="app-shell">
        <nav className="app-nav">
          <span className="app-title">♟ Harland Chess Trainer</span>
          <div className="nav-links">
            <NavLink to="/" end>
              Sync
            </NavLink>
            <NavLink to="/settings">Settings</NavLink>
          </div>
        </nav>
        <main className="app-main">
          <Routes>
            <Route path="/" element={<SyncPage />} />
            <Route path="/settings" element={<SettingsPage />} />
          </Routes>
        </main>
      </div>
    </HashRouter>
  );
}

export default App;

