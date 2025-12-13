/// <reference types="vite/client" />

interface ImportMetaEnv {
  readonly VITE_API_URL: string;
  readonly VITE_WS_URL: string;
  readonly VITE_AUTH_ENABLED: string;
  readonly VITE_ENABLE_RAC: string;
  readonly VITE_ENABLE_FLASHBACK: string;
  readonly VITE_ENABLE_STREAMING: string;
  readonly VITE_ENABLE_ML: string;
  readonly VITE_ENABLE_SPATIAL: string;
  readonly VITE_ENABLE_GRAPH: string;
  readonly VITE_ENABLE_TIMESERIES: string;
  readonly VITE_ENABLE_BLOCKCHAIN: string;
  readonly VITE_ENABLE_FEDERATION: string;
  readonly VITE_ENVIRONMENT: string;
  readonly NODE_ENV: string;
  readonly MODE: string;
  readonly PROD: boolean;
  readonly DEV: boolean;
  readonly SSR: boolean;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}
