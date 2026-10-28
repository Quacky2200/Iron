// SPDX-License-Identifier: Apache-2.0

// The crash dialog: Platypus's own window for a game that crashed, run as a small game of its own by
// the crash supervisor (`crash_dialog.rs`). The supervisor puts `GAME`, `REASON`, `DETAILS` and
// `REPORT_PATH` (string constants) in front of this text. The answer is the exit code: 0 closes,
// 10 also opens the report's folder.

const OPEN_FOLDER = 10;

function Button(props: { label: string; primary: boolean; onPress: () => void }) {
  return (
    <box
      style={{
        height: 34,
        paddingLeft: 18,
        paddingRight: 18,
        marginLeft: 10,
        borderRadius: 8,
        alignItems: "center",
        justifyContent: "center",
        backgroundColor: props.primary ? "#d23a32" : "#30333b",
        borderWidth: 1,
        borderColor: props.primary ? "#e5554c" : "#41454f",
        cursor: "pointer",
        flexShrink: 0,
      }}
      onClick={() => props.onPress()}
    >
      <text style={{ color: "#ffffff", fontSize: 14, whiteSpace: "nowrap" }}>{props.label}</text>
    </box>
  );
}

function Dialog() {
  return (
    <box style={{ width: "100%", height: "100%", flexDirection: "column", padding: 24, backgroundColor: "#1b1c21" }}>
      <box style={{ flexDirection: "row", alignItems: "center" }}>
        <box
          style={{
            width: 36,
            height: 36,
            borderRadius: 18,
            marginRight: 14,
            backgroundColor: "#d23a32",
            alignItems: "center",
            justifyContent: "center",
          }}
        >
          <text style={{ color: "#ffffff", fontSize: 22 }}>!</text>
        </box>
        <text style={{ color: "#ffffff", fontSize: 20 }}>{`${GAME} stopped unexpectedly`}</text>
      </box>
      <text style={{ color: "#b6bac4", fontSize: 14, marginTop: 14 }}>
        It ran into an error it could not recover from and had to close. A crash report was saved.
      </text>
      <box
        style={{
          marginTop: 14,
          padding: 10,
          borderRadius: 6,
          backgroundColor: "#2a1d1f",
          borderLeftWidth: 3,
          borderColor: "#d23a32",
        }}
      >
        <text style={{ color: "#f3c9c5", fontSize: 13 }}>{REASON}</text>
      </box>
      <text style={{ color: "#868b96", fontSize: 12, marginTop: 16, marginBottom: 6 }}>DETAILS</text>
      {/* A scrolling box, not a text area: it opens at the report's top (its summary). */}
      <box
        style={{
          flexGrow: 1,
          flexShrink: 1,
          overflow: "auto",
          padding: 8,
          borderRadius: 6,
          borderWidth: 1,
          borderColor: "#2e3038",
          backgroundColor: "#14151a",
        }}
      >
        <text style={{ color: "#c9ccd3", fontSize: 12, whiteSpace: "pre" }}>{DETAILS}</text>
      </box>
      <box style={{ flexDirection: "row", alignItems: "center", marginTop: 16 }}>
        <text style={{ color: "#6f7480", fontSize: 11, flexGrow: 1, flexShrink: 1 }}>{REPORT_PATH}</text>
        <Button label="Open report folder" primary={false} onPress={() => Application.quit(OPEN_FOLDER)} />
        <Button label="Close" primary={true} onPress={() => Application.quit(0)} />
      </box>
    </box>
  );
}

createRoot().render(<Dialog />);
