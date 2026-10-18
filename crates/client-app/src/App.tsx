// src/App.tsx — Main React App
import { useState } from "react";
import { ConnectScreen } from "./components/ConnectScreen";
import { RemoteView } from "./components/RemoteView";
import { Toolbar } from "./components/Toolbar";

export type ConnectionState =
  | { status: "disconnected" }
  | { status: "connecting"; pin: string }
  | { status: "connected"; sessionId: string; peerIp: string; connType: string }
  | { status: "error"; message: string };

function App() {
  const [connection, setConnection] = useState<ConnectionState>({
    status: "disconnected",
  });

  return (
    <div className="flex flex-col h-screen bg-gray-950 text-white">
      {connection.status === "connected" ? (
        <>
          <Toolbar
            sessionId={connection.sessionId}
            peerIp={connection.peerIp}
            connType={connection.connType}
            onDisconnect={() => setConnection({ status: "disconnected" })}
          />
          <RemoteView
            sessionId={connection.sessionId}
            onDisconnected={() => setConnection({ status: "disconnected" })}
          />
        </>
      ) : (
        <ConnectScreen
          state={connection}
          onConnect={(pin) => setConnection({ status: "connecting", pin })}
          onConnected={(sessionId, peerIp, connType) =>
            setConnection({ status: "connected", sessionId, peerIp, connType })
          }
          onError={(msg) => setConnection({ status: "error", message: msg })}
        />
      )}
    </div>
  );
}

export default App;

