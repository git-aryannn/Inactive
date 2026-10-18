// src/components/ConnectScreen.tsx
// PIN entry screen — first screen user sees

import { useState, useRef, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { ConnectionState } from "../App";

interface Props {
  state: ConnectionState;
  onConnect: (pin: string) => void;
  onConnected: (sessionId: string, peerIp: string, connType: string) => void;
  onError: (msg: string) => void;
}

export function ConnectScreen({ state, onConnect, onConnected, onError }: Props) {
  const [digits, setDigits] = useState<string[]>(() => {
    try {
      const saved = localStorage.getItem("inactive_last_pin");
      if (saved && saved.length === 9) {
        return saved.split("");
      }
    } catch (_) {}
    return ["", "", "", "", "", "", "", "", ""];
  });
  const inputRefs = useRef<(HTMLInputElement | null)[]>([]);

  // Focus first input on mount
  useEffect(() => {
    inputRefs.current[0]?.focus();
  }, []);

  // Listen for connection events from Tauri backend
  useEffect(() => {
    const unlisten = listen<string>("connection_status", (event) => {
      if (event.payload.startsWith("connected:")) {
        const parts = event.payload.split(":");
        onConnected(parts[1], parts[2], parts[3]);
      } else if (event.payload.startsWith("error:")) {
        onError(event.payload.slice(6));
      }
    });
    return () => { unlisten.then((f) => f()); };
  }, []);

  const pin = digits.join("");
  const isComplete = pin.length === 9;

  const handleDigitChange = (index: number, value: string) => {
    // Only accept digits
    const digit = value.replace(/\D/g, "").slice(-1);
    const newDigits = [...digits];
    newDigits[index] = digit;
    setDigits(newDigits);

    // Auto-advance to next input
    if (digit && index < 8) {
      inputRefs.current[index + 1]?.focus();
    }
  };

  const handleKeyDown = (index: number, e: React.KeyboardEvent) => {
    // Backspace goes to previous input
    if (e.key === "Backspace" && !digits[index] && index > 0) {
      inputRefs.current[index - 1]?.focus();
    }
    // Enter submits if complete
    if (e.key === "Enter" && isComplete) {
      handleConnect();
    }
  };

  const handlePaste = (e: React.ClipboardEvent) => {
    e.preventDefault();
    const pasted = e.clipboardData.getData("text").replace(/\D/g, "").slice(0, 9);
    const newDigits = [...digits];
    for (let i = 0; i < pasted.length; i++) {
      newDigits[i] = pasted[i];
    }
    setDigits(newDigits);
    // Focus appropriate input
    inputRefs.current[Math.min(pasted.length, 8)]?.focus();
  };

  const handleConnect = async () => {
    if (!isComplete) return;
    try {
      localStorage.setItem("inactive_last_pin", pin);
    } catch (_) {}
    onConnect(pin);
    try {
      await invoke("connect_to_host", { pin });
    } catch (e) {
      onError(String(e));
    }
  };

  return (
    <div className="flex flex-col items-center justify-center flex-1 gap-10 p-8">
      {/* Logo / Title */}
      <div className="text-center">
        <h1 className="text-4xl font-bold bg-gradient-to-r from-blue-400 to-cyan-400 bg-clip-text text-transparent">
          Inactive
        </h1>
        <p className="text-gray-400 mt-2">Enter the 9-digit PIN from the host computer</p>
      </div>

      {/* PIN Input Grid */}
      <div className="flex gap-2 items-center">
        {digits.map((digit, i) => (
          <div key={i} className="flex items-center gap-2">
            <input
              ref={(el) => (inputRefs.current[i] = el)}
              type="text"
              inputMode="numeric"
              maxLength={1}
              value={digit}
              onChange={(e) => handleDigitChange(i, e.target.value)}
              onKeyDown={(e) => handleKeyDown(i, e)}
              onPaste={handlePaste}
              className={`
                w-12 h-14 text-center text-2xl font-mono font-bold rounded-xl
                border-2 bg-gray-800 outline-none transition-all duration-150
                ${digit
                  ? "border-blue-500 text-white"
                  : "border-gray-600 text-gray-400 placeholder-gray-600"
                }
                focus:border-blue-400 focus:ring-2 focus:ring-blue-400/20
                caret-blue-400
              `}
              placeholder="•"
            />
            {/* Dashes at positions 3 and 6 */}
            {(i === 2 || i === 5) && (
              <span className="text-gray-500 text-xl font-bold select-none">—</span>
            )}
          </div>
        ))}
      </div>

      {/* Status / Error */}
      {state.status === "error" && (
        <div className="bg-red-900/40 border border-red-500/50 text-red-300 px-6 py-3 rounded-xl text-sm">
          ⚠️ {state.message}
        </div>
      )}

      {state.status === "connecting" && (
        <div className="flex items-center gap-3 text-blue-300">
          <div className="w-5 h-5 border-2 border-blue-400 border-t-transparent rounded-full animate-spin" />
          Connecting to host...
        </div>
      )}

      {/* Connect Button */}
      <button
        onClick={handleConnect}
        disabled={!isComplete || state.status === "connecting"}
        className={`
          px-10 py-3 rounded-xl font-semibold text-lg transition-all duration-200
          ${isComplete && state.status !== "connecting"
            ? "bg-blue-600 hover:bg-blue-500 active:scale-95 text-white shadow-lg shadow-blue-500/25"
            : "bg-gray-700 text-gray-500 cursor-not-allowed"
          }
        `}
      >
        {state.status === "connecting" ? "Connecting..." : "Connect"}
      </button>

      {/* Help text */}
      <p className="text-gray-600 text-sm text-center max-w-sm">
        Make sure the Inactive agent is running on the host machine and
        share the PIN shown there.
      </p>
    </div>
  );
}

