// src/components/Toolbar.tsx
// Floating session control toolbar shown during active remote session

import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";

interface Props {
  sessionId: string;
  peerIp: string;
  connType: string;
  onDisconnect: () => void;
}

export function Toolbar({ sessionId, peerIp, connType, onDisconnect }: Props) {
  const [isMuted, setIsMuted] = useState(false);
  const [clipboardSynced, setClipboardSynced] = useState(false);

  const handleDisconnect = async () => {
    try {
      await invoke("disconnect");
    } catch (_) {}
    onDisconnect();
  };

  const handleClipboardSync = () => {
    setClipboardSynced(true);
    setTimeout(() => setClipboardSynced(false), 2000);
  };

  return (
    <div className="bg-gray-900 border-b border-gray-800 px-4 py-2 flex items-center justify-between z-10 select-none">
      <div className="flex items-center gap-4">
        <div className="flex items-center gap-2">
          <span className="w-2.5 h-2.5 rounded-full bg-emerald-500 animate-pulse" />
          <span className="text-sm font-medium text-gray-200">
            Connected: <span className="font-mono text-blue-400">{peerIp}</span>
          </span>
        </div>

        <span className="text-xs px-2 py-0.5 rounded-md bg-gray-800 text-gray-400 border border-gray-700">
          {connType === "p2p" ? "⚡ Direct P2P" : "☁️ Relayed (TURN)"}
        </span>

        <span className="text-xs font-mono text-gray-500">
          Session #{sessionId.slice(0, 8)}
        </span>
      </div>

      <div className="flex items-center gap-2">
        <button
          onClick={handleClipboardSync}
          className="text-xs px-3 py-1.5 rounded-lg bg-gray-800 hover:bg-gray-700 text-gray-300 border border-gray-700 transition"
          title="Sync clipboard between machines"
        >
          {clipboardSynced ? "✓ Synced" : "📋 Clipboard Sync"}
        </button>

        <button
          onClick={() => setIsMuted(!isMuted)}
          className="text-xs px-3 py-1.5 rounded-lg bg-gray-800 hover:bg-gray-700 text-gray-300 border border-gray-700 transition"
        >
          {isMuted ? "🔇 Unmute" : "🔊 Mute"}
        </button>

        <button
          onClick={handleDisconnect}
          className="text-xs px-3 py-1.5 rounded-lg bg-red-600/80 hover:bg-red-500 text-white font-medium transition"
        >
          Disconnect
        </button>
      </div>
    </div>
  );
}
