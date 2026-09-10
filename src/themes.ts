export interface Palette {
  background: string;
  surface: string;
  foreground: string;
  muted: string;
  accent: string;
  border: string;
}
export interface Theme {
  id: string;
  name: string;
  colors: Palette;
  custom?: boolean;
}
const values = [
  [
    "ulix-dark",
    "Dark",
    "#000000",
    "#1e1e1e",
    "#ffffff",
    "#737373",
    "#10b981",
    "#1a1a1a",
  ],
  [
    "ulix-light",
    "Light",
    "#ffffff",
    "#f5f5f5",
    "#000000",
    "#737373",
    "#059669",
    "#e5e5e5",
  ],
  [
    "nord",
    "Nord",
    "#2e3440",
    "#3b4252",
    "#eceff4",
    "#a3b2c7",
    "#88c0d0",
    "#4c566a",
  ],
  [
    "dracula",
    "Dracula",
    "#282a36",
    "#343746",
    "#f8f8f2",
    "#a6a6c9",
    "#bd93f9",
    "#44475a",
  ],
  [
    "solarized-dark",
    "Solarized Dark",
    "#002b36",
    "#073642",
    "#eee8d5",
    "#93a1a1",
    "#2aa198",
    "#204954",
  ],
  [
    "solarized-light",
    "Solarized Light",
    "#fdf6e3",
    "#eee8d5",
    "#073642",
    "#657b83",
    "#268b8a",
    "#d4cdbb",
  ],
  [
    "catppuccin",
    "Catppuccin Mocha",
    "#1e1e2e",
    "#313244",
    "#cdd6f4",
    "#a6adc8",
    "#cba6f7",
    "#45475a",
  ],
  [
    "gruvbox",
    "Gruvbox",
    "#282828",
    "#3c3836",
    "#ebdbb2",
    "#bdae93",
    "#b8bb26",
    "#504945",
  ],
  [
    "tokyo",
    "Tokyo Night",
    "#1a1b26",
    "#24283b",
    "#c0caf5",
    "#9aa5ce",
    "#7aa2f7",
    "#3b4261",
  ],
  [
    "rose-pine",
    "Rosé Pine",
    "#191724",
    "#26233a",
    "#e0def4",
    "#908caa",
    "#ebbcba",
    "#403d52",
  ],
  [
    "midnight",
    "Midnight",
    "#090d13",
    "#121c26",
    "#e5edf5",
    "#8698ac",
    "#64d8cb",
    "#243340",
  ],
  [
    "everforest",
    "Everforest",
    "#2d353b",
    "#343f44",
    "#d3c6aa",
    "#a7b7a3",
    "#a7c080",
    "#475258",
  ],
];
export const presets: Theme[] = values.map(
  ([id, name, background, surface, foreground, muted, accent, border]) => ({
    id,
    name,
    colors: { background, surface, foreground, muted, accent, border },
  }),
);
const valid = (v: unknown): v is Theme => {
  if (!v || typeof v !== "object") return false;
  const t = v as Partial<Theme>;
  return (
    typeof t.id === "string" &&
    typeof t.name === "string" &&
    t.name.length <= 60 &&
    !!t.colors &&
    Object.keys(presets[0].colors).every((k) =>
      /^#[0-9a-fA-F]{6}$/.test(t.colors?.[k as keyof Palette] ?? ""),
    )
  );
};
export function readThemes(): Theme[] {
  try {
    const raw: unknown = JSON.parse(
      localStorage.getItem("lanyard.custom-themes.v2") ?? "[]",
    );
    return Array.isArray(raw)
      ? raw
          .filter(valid)
          .slice(0, 30)
          .map((t) => ({ ...t, custom: true }))
      : [];
  } catch {
    return [];
  }
}
export function applyTheme(t: Theme) {
  for (const [k, v] of Object.entries(t.colors))
    document.documentElement.style.setProperty(
      k === "muted" ? "--muted-foreground" : `--${k}`,
      v,
    );
  document.documentElement.style.colorScheme = t.id.includes("light")
    ? "light"
    : "dark";
  try {
    localStorage.setItem("lanyard.theme.v2", t.id);
  } catch {
    /* Palette still applies for this session. */
  }
}
export function initialTheme(): Theme {
  let id: string | null = null;
  try {
    id = localStorage.getItem("lanyard.theme.v2");
  } catch {
    /* Use ULIX Dark. */
  }
  return [...presets, ...readThemes()].find((t) => t.id === id) ?? presets[0];
}
