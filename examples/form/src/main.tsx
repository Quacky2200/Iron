// SPDX-License-Identifier: Apache-2.0

// M3c showcase: interaction. A profile form with controlled text fields (validation,
// a character counter), keyboard- and gamepad-operable toggles and buttons with
// :hover / :active / :focus-visible / :disabled styles, and a scrolling list of saved
// profiles. Tab / Shift-Tab move focus; with a gamepad, the d-pad or left stick moves
// focus and A (South) clicks. The name field has focus at start, so the d-pad works
// straight away. The manual checks M3-MAN-01 use this example.
import "./form.css";


interface Profile {
  name: string;
  email: string;
  bio: string;
  newsletter: boolean;
  publicProfile: boolean;
}

const EMPTY: Profile = { name: "", email: "", bio: "", newsletter: true, publicProfile: false };
const BIO_LIMIT = 140;

const SAMPLES: Profile[] = [
  "Ada Lovelace",
  "Alan Turing",
  "Grace Hopper",
  "Edsger Dijkstra",
  "Barbara Liskov",
  "Donald Knuth",
  "Margaret Hamilton",
  "Ken Thompson",
  "Frances Allen",
  "John Carmack",
  "Radia Perlman",
  "Linus Torvalds",
  "Hedy Lamarr",
  "Claude Shannon",
  "Katherine Johnson",
  "Dennis Ritchie",
  "Sophie Wilson",
  "Bjarne Stroustrup",
  "Mary Kenneth Keller",
  "Tim Berners-Lee",
  "Annie Easley",
  "Guido van Rossum",
].map((name) => ({
  ...EMPTY,
  name,
  email: `${name.split(" ")[0]!.toLowerCase()}@example.com`,
  bio: `Notes about ${name}.`,
}));

const validEmail = (email: string) => /^[^@\s]+@[^@\s]+\.[^@\s]+$/.test(email);

/** A focusable box that clicks with the mouse, Enter / Space, or a gamepad's A. */
function Button({
  children,
  onPress,
  disabled = false,
  primary = false,
}: {
  children: ReactNode;
  onPress: () => void;
  disabled?: boolean;
  primary?: boolean;
}) {
  const press = () => !disabled && onPress();
  return (
    <box
      className={primary ? "button primary" : "button"}
      tabIndex={0}
      disabled={disabled}
      onClick={press}
      onKeyDown={(e: PlatypusKeyboardEvent) => {
        if (e.key === "Enter" || e.key === " ") {
          e.preventDefault();
          press();
        }
      }}
    >
      <Text>{children}</Text>
    </box>
  );
}

function Toggle({ label, checked, onChange }: { label: string; checked: boolean; onChange: (v: boolean) => void }) {
  return (
    <box
      className="toggle"
      tabIndex={0}
      checked={checked}
      onClick={() => onChange(!checked)}
      onKeyDown={(e: PlatypusKeyboardEvent) => {
        if (e.key === " " || e.key === "Enter") {
          e.preventDefault();
          onChange(!checked);
        }
      }}
    >
      <box className="check" />
      <Text>{label}</Text>
    </box>
  );
}

function Field({ label, error, children }: { label: string; error?: string; children: ReactNode }) {
  return (
    <box className="field">
      <Text className="label">{label}</Text>
      {children}
      {error ? <Text className="error">{error}</Text> : null}
    </box>
  );
}

function usePads(): Gamepad[] {
  const [pads, setPads] = useState<Gamepad[]>(() => input.gamepads());
  useEffect(() => {
    const update = () => setPads(input.gamepads());
    const stops = [input.on("gamepadconnected", update), input.on("gamepaddisconnected", update)];
    return () => stops.forEach((stop) => stop());
  }, []);
  return pads;
}

function App() {
  const [form, setForm] = useState<Profile>(EMPTY);
  const [saved, setSaved] = useState<Profile[]>(SAMPLES);
  const [status, setStatus] = useState("Fill in the form and press Save.");
  const [touched, setTouched] = useState(false);
  const nameRef = useRef<PlatypusElement>(null);
  const listRef = useRef<PlatypusElement>(null);
  const pads = usePads();

  useEffect(() => nameRef.current?.focus(), []);

  const set = <K extends keyof Profile>(key: K, value: Profile[K]) => setForm((f) => ({ ...f, [key]: value }));
  const nameError = touched && !form.name.trim() ? "A name is required." : undefined;
  const emailError = touched && !validEmail(form.email) ? "Enter an email like ada@example.com." : undefined;
  const valid = form.name.trim() !== "" && validEmail(form.email);

  const save = () => {
    setTouched(true);
    if (!valid) {
      setStatus("Fix the highlighted fields first.");
      return;
    }
    setSaved((list) => [...list.filter((p) => p.name !== form.name), form]);
    setStatus(`Saved ${form.name}.`);
    // Show the new entry at the end of the list.
    listRef.current?.scrollTo(0, 1e9);
  };
  const reset = () => {
    setForm(EMPTY);
    setTouched(false);
    setStatus("Cleared.");
    nameRef.current?.focus();
  };

  return (
    <box className="app">
      <box className="header">
        <Text className="title">Profile</Text>
        <Text className="pads">
          {pads.length ? `Gamepad: ${pads.map((p) => p.name).join(", ")}` : "No gamepad connected"}
        </Text>
      </box>

      <box className="columns">
        <box
          className="card form"
          onKeyDown={(e: PlatypusKeyboardEvent) => {
            // Enter submits from a one-line field (textareas take Enter for new lines).
            if (e.key === "Enter" && e.target.type === "input") save();
          }}
        >
          <Field label="NAME" error={nameError}>
            <Input
              ref={nameRef}
              className={nameError ? "invalid" : ""}
              placeholder="Ada Lovelace"
              value={form.name}
              onChange={(e) => set("name", e.target.value ?? "")}
            />
          </Field>
          <Field label="EMAIL" error={emailError}>
            <Input
              className={emailError ? "invalid" : ""}
              placeholder="ada@example.com"
              value={form.email}
              onChange={(e) => set("email", e.target.value ?? "")}
            />
          </Field>
          <Field label={`BIO  ${form.bio.length}/${BIO_LIMIT}`}>
            <TextArea
              rows={4}
              placeholder="A few words about you. Enter adds a line."
              value={form.bio}
              onChange={(e) => {
                const v = e.target.value ?? "";
                if (v.length <= BIO_LIMIT) set("bio", v);
              }}
            />
          </Field>
          <box className="toggles">
            <Toggle label="Send me the newsletter" checked={form.newsletter} onChange={(v) => set("newsletter", v)} />
            <Toggle label="Public profile" checked={form.publicProfile} onChange={(v) => set("publicProfile", v)} />
          </box>
          <box className="actions">
            <Button onPress={reset}>Reset</Button>
            <Button primary disabled={touched && !valid} onPress={save}>
              Save
            </Button>
          </box>
          <Text className="status">{status}</Text>
        </box>

        <box className="card saved">
          <Text className="label">{`SAVED PROFILES (${saved.length})`}</Text>
          <box className="list" ref={listRef}>
            {saved.map((p) => (
              <box
                key={p.name}
                className={p.name === form.name ? "row current" : "row"}
                tabIndex={0}
                onClick={() => {
                  setForm(p);
                  setStatus(`Loaded ${p.name}.`);
                }}
              >
                <Text className="row-name">{p.name}</Text>
                <Text className="row-email">{p.email}</Text>
              </box>
            ))}
          </box>
        </box>
      </box>

      <Text className="hint">
        Tab / Shift+Tab move focus · Enter or Space press · wheel, keys or the scrollbar scroll · gamepad: d-pad
        moves, A presses
      </Text>
    </box>
  );
}

createRoot().render(<App />);
