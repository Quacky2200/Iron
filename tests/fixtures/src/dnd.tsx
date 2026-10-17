// M5-DND-09: drag and drop from Rust through React. The item is draggable and puts
// its name in the dataTransfer; the folder accepts drops (prevents dragover) and shows
// what it got, including files dragged in from the system. The status line says how
// the drag ended.
import { createRoot, type IronDragEvent, Text } from "@iron/react";
import { useState } from "react";

const at = (left: number, top: number) => ({ position: "absolute" as const, left, top, width: 50, height: 50 });

function App() {
  const [got, setGot] = useState("nothing");
  const [status, setStatus] = useState("idle");
  return (
    <box style={{ width: "100%", height: "100%" }}>
      <box
        draggable
        style={at(0, 0)}
        onDragStart={(e: IronDragEvent) => {
          e.dataTransfer.setData("text/plain", "item");
          setStatus("dragging");
        }}
        onDragEnd={(e: IronDragEvent) => setStatus(e.dropped ? "dropped" : "cancelled")}
      />
      <box
        style={at(100, 0)}
        onDragOver={(e: IronDragEvent) => e.preventDefault()}
        onDrop={(e: IronDragEvent) =>
          setGot(e.dataTransfer.files.length ? e.dataTransfer.files.map((f) => f.name).join(",") : e.dataTransfer.getData("text/plain"))
        }
      />
      <box style={{ position: "absolute", left: 0, top: 100, flexDirection: "column" }}>
        <Text>{got}</Text>
        <Text>{status}</Text>
      </box>
    </box>
  );
}

createRoot().render(<App />);
