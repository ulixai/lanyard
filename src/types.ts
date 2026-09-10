export type Category =
  "api_key" | "password" | "license" | "recovery" | "env" | "crypto";
export type Fields = Record<string, string>;
export interface Project {
  id: string;
  title: string;
  description: string;
}
export interface Item {
  id: string;
  title: string;
  category: Category;
  project_id: string | null;
  fields: string[];
}
export interface Snapshot {
  initialized: boolean;
  locked: boolean;
  projects: Project[];
  items: Item[];
  clients: { id: string; name: string }[];
  grants: { client_id: string; item_id: string }[];
  close_to_tray: boolean;
  legacy_available: boolean;
  port: number;
}
export interface AccessRequest {
  id: string;
  client_id: string;
  app_name: string;
  target_id: string | null;
  category: Category | null;
  reason: string | null;
  paired: boolean;
  expires_in: number;
}
export const categories: {
  id: Category;
  name: string;
  singular: string;
  fields: string[];
}[] = [
  { id: "api_key", name: "API keys", singular: "API key", fields: ["API_KEY"] },
  {
    id: "password",
    name: "Passwords",
    singular: "Password",
    fields: ["Username", "Password", "URL"],
  },
  {
    id: "license",
    name: "Licenses",
    singular: "License",
    fields: ["License Key", "Registered Email"],
  },
  {
    id: "recovery",
    name: "Recovery codes",
    singular: "Recovery codes",
    fields: ["Account", "Recovery Codes"],
  },
  {
    id: "env",
    name: "Environments",
    singular: "Environment",
    fields: ["API_KEY"],
  },
  {
    id: "crypto",
    name: "Cryptographic keys",
    singular: "Key pair",
    fields: ["private_key", "public_key"],
  },
];
export const categoryName = (id: Category) =>
  categories.find((c) => c.id === id)?.singular ?? id;
