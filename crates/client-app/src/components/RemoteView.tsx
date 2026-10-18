// src/components/RemoteView.tsx
// Main remote desktop canvas — renders video stream and captures input

import { useEffect, useRef, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

interface Props {
  sessionId: string;
  onDisconnected: () => void;
}

export function RemoteView({ sessionId, onDisconnected }: Props) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);

  // Listen for video frames from backend
  useEffect(() => {
    // Backend emits "video_frame" events with base64-encoded JPEG data
    const unlisten = listen<{ data: number[]; width: number; height: number }>(
      "video_frame",
      (event) => {
        const canvas = canvasRef.current;
        if (!canvas) return;

        const ctx = canvas.getContext("2d");
        if (!ctx) return;

        const { data, width, height } = event.payload;

        // Update canvas size if changed
        if (canvas.width !== width || canvas.height !== height) {
          canvas.width = width;
          canvas.height = height;
        }

        // Draw decoded frame
        const imageData = new ImageData(
          new Uint8ClampedArray(data),
          width,
          height
        );
        ctx.putImageData(imageData, 0, 0);
      }
    );

    // Listen for disconnect
    const unlistenDisc = listen("session_disconnected", () => {
      onDisconnected();
    });

    return () => {
      unlisten.then((f) => f());
      unlistenDisc.then((f) => f());
    };
  }, [sessionId]);

  // Convert canvas coordinates to normalized (0.0-1.0) remote coordinates
  const toNormalized = useCallback(
    (e: React.MouseEvent<HTMLCanvasElement>) => {
      const canvas = canvasRef.current;
      if (!canvas) return { x: 0, y: 0 };
      const rect = canvas.getBoundingClientRect();
      return {
        x: (e.clientX - rect.left) / rect.width,
        y: (e.clientY - rect.top) / rect.height,
      };
    },
    []
  );

  // ── Mouse Event Handlers ─────────────────────────────────────────────────

  const handleMouseMove = useCallback(
    (e: React.MouseEvent<HTMLCanvasElement>) => {
      const { x, y } = toNormalized(e);
      invoke("send_mouse_event", { x, y }).catch(() => {});
    },
    [toNormalized]
  );

  const handleMouseDown = useCallback(
    (e: React.MouseEvent<HTMLCanvasElement>) => {
      e.preventDefault();
      const { x, y } = toNormalized(e);
      const button = ["Left", "Middle", "Right"][e.button] ?? "Left";
      invoke("send_mouse_event", { x, y, button, pressed: true }).catch(() => {});
    },
    [toNormalized]
  );

  const handleMouseUp = useCallback(
    (e: React.MouseEvent<HTMLCanvasElement>) => {
      const { x, y } = toNormalized(e);
      const button = ["Left", "Middle", "Right"][e.button] ?? "Left";
      invoke("send_mouse_event", { x, y, button, pressed: false }).catch(() => {});
    },
    [toNormalized]
  );

  const handleWheel = useCallback(
    (e: React.WheelEvent<HTMLCanvasElement>) => {
      e.preventDefault();
      invoke("send_mouse_event", {
        x: 0,
        y: 0,
        scrollDelta: -e.deltaY / 100,
      }).catch(() => {});
    },
    []
  );

  const handleContextMenu = useCallback(
    (e: React.MouseEvent) => e.preventDefault(),
    []
  );

  // ── Keyboard Event Handlers ──────────────────────────────────────────────

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent<HTMLCanvasElement>) => {
      e.preventDefault();
      invoke("send_key_event", { keyCode: e.keyCode, pressed: true }).catch(() => {});
    },
    []
  );

  const handleKeyUp = useCallback(
    (e: React.KeyboardEvent<HTMLCanvasElement>) => {
      e.preventDefault();
      invoke("send_key_event", { keyCode: e.keyCode, pressed: false }).catch(() => {});
    },
    []
  );

  return (
    <div
      ref={containerRef}
      className="flex-1 flex items-center justify-center bg-black overflow-hidden"
    >
      <canvas
        ref={canvasRef}
        tabIndex={0}
        onMouseMove={handleMouseMove}
        onMouseDown={handleMouseDown}
        onMouseUp={handleMouseUp}
        onWheel={handleWheel}
        onContextMenu={handleContextMenu}
        onKeyDown={handleKeyDown}
        onKeyUp={handleKeyUp}
        className="max-w-full max-h-full cursor-none outline-none"
        style={{ imageRendering: "pixelated" }}
      />
    </div>
  );
}

