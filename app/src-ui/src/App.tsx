import { useState } from "react";
import { syncGames, SyncResult } from "./api/lichess";
import "./App.css";

function App() {
  const [username, setUsername] = useState("");
  const [maxGames, setMaxGames] = useState(50);
  const [result, setResult] = useState<SyncResult | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  async function handleSync() {
    if (!username.trim()) return;
    setLoading(true);
    setError(null);
    setResult(null);
    try {
      const res = await syncGames(username.trim(), maxGames);
      setResult(res);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }

  return (
    <main className="container">
      <h1>Harland Chess Trainer</h1>
      <p>Fetch your Lichess games to get started.</p>

      <form
        className="row"
        onSubmit={(e) => {
          e.preventDefault();
          handleSync();
        }}
      >
        <input
          value={username}
          onChange={(e) => setUsername(e.currentTarget.value)}
          placeholder="Lichess username"
          disabled={loading}
        />
        <input
          type="number"
          value={maxGames}
          onChange={(e) => setMaxGames(Number(e.currentTarget.value))}
          min={1}
          max={300}
          style={{ width: "80px" }}
          disabled={loading}
        />
        <button type="submit" disabled={loading || !username.trim()}>
          {loading ? "Syncing…" : "Sync Games"}
        </button>
      </form>

      {error && <p style={{ color: "red" }}>Error: {error}</p>}

      {result && (
        <div style={{ marginTop: "1rem" }}>
          <p>Fetched: {result.fetched}</p>
          <p>New: {result.new}</p>
          <p>Updated: {result.updated}</p>
        </div>
      )}
    </main>
  );
}

export default App;
