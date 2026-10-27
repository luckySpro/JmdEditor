/// <reference types="vite/client" />
declare module '*.vue' {
  import type { DefineComponent } from 'vue'
  const component: DefineComponent<object, object, unknown>
  export default component
}
interface ImportMetaEnv {
  /** Base URL of jmd-server when the UI runs in a browser, e.g. https://export.example.com */
  readonly VITE_API_BASE?: string
}
