import ConnectionStatus from "./components/ConnectionStatus";
import SimulationControls from "./components/SimulationControls";
import WorldViewport from "./components/WorldViewport";
import InspectorPanel from "./components/InspectorPanel";
import TimelinePanel from "./components/TimelinePanel";
import ExplanationPanel from "./components/ExplanationPanel";

function App() {
  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100vh" }}>
      <header style={{ padding: "8px 16px", borderBottom: "1px solid #ccc" }}>
        <h1>Ontopolis Observer</h1>
        <ConnectionStatus />
      </header>
      <main style={{ flex: 1, display: "flex", overflow: "hidden" }}>
        <aside style={{ width: 250, borderRight: "1px solid #ccc", padding: 8 }}>
          <SimulationControls />
          <TimelinePanel />
        </aside>
        <section style={{ flex: 1, padding: 8 }}>
          <WorldViewport />
        </section>
        <aside style={{ width: 300, borderLeft: "1px solid #ccc", padding: 8 }}>
          <InspectorPanel />
          <ExplanationPanel />
        </aside>
      </main>
    </div>
  );
}

export default App;
